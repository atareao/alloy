use bollard::{
    container::RestartContainerOptions,
    image::PruneImagesOptions,
    Docker,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

use crate::containers::{find_container_by_name, pull_image};
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::handlers::{log_prune_result, prune_dangling_images};
use crate::workers::state::docker_list_running;

/// Auto-update worker: periodic pull + restart según políticas
pub async fn auto_update_worker(
    docker: Docker,
    settings: Arc<Mutex<Settings>>,
    update_policies: Arc<Mutex<Vec<UpdatePolicy>>>,
    notif_tx: broadcast::Sender<NotifEvent>,
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: DbPool,
) {
    // Esperar al startup antes de la primera ejecución
    tokio::time::sleep(Duration::from_secs(300)).await; // 5 min

    loop {
        // Leer configuración en cada ciclo (para soportar cambios dinámicos)
        let (enabled, interval_hours) = {
            let s = settings.lock().await;
            (
                s.auto_update_enabled.unwrap_or(false),
                s.auto_update_interval_hours.unwrap_or(6).max(1),
            )
        };
        if !enabled {
            tokio::time::sleep(Duration::from_secs(60)).await;
            continue;
        }

        // Esperar el intervalo ANTES de procesar (no al crear el interval)
        tokio::time::sleep(Duration::from_secs(interval_hours * 3600)).await;

        // Cargar políticas una vez por ciclo
        let policies = {
            let p = update_policies.lock().await;
            let map: HashMap<String, UpdatePolicy> = p
                .iter()
                .map(|pol| (pol.container.clone(), pol.clone()))
                .collect();
            map
        };

        for (name, image, cid, image_id) in docker_list_running(&docker).await {
            let should_update = match crate::updates::digest::check_remote_digest(&image).await {
                Ok((remote_digest, _)) => {
                    let has_update = image_id.as_ref().is_none_or(|local_digest| {
                        let local_short = local_digest
                            .split(':')
                            .next_back()
                            .unwrap_or("")
                            .chars()
                            .take(12)
                            .collect::<String>();
                        let remote_short = remote_digest
                            .split(':')
                            .next_back()
                            .unwrap_or("")
                            .chars()
                            .take(12)
                            .collect::<String>();
                        local_short != remote_short
                    });
                    has_update
                }
                Err(e) => {
                    tracing::warn!(
                        "auto_update [{}]: error verificando digest remoto: {}, NO se hará pull",
                        name,
                        e
                    );
                    false
                }
            };

            if !should_update {
                continue;
            }

            // Leer política para este contenedor
            let policy = policies.get(&name).cloned().unwrap_or(UpdatePolicy {
                container: name.clone(),
                action: UpdateAction::PullRestart,
                cleanup_old_image: false,
                rollback_on_failure: false,
                notify_events: false,
            });

            if policy.action == UpdateAction::None {
                continue;
            }

            let start_time = std::time::Instant::now();
            let pull_timeout = {
                let s = settings.lock().await;
                s.pull_timeout_secs.unwrap_or(600)
            };
            tracing::info!(
                "auto_update: descargando '{}' (imagen: {}, timeout: {}s)",
                name,
                image,
                pull_timeout
            );
            if !pull_image(&docker, &image, pull_timeout).await {
                tracing::error!("auto_update: pull FALLÓ para '{}'", name);
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest: String::new(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "auto-update-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let obj = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&obj.lock().unwrap(), hist.last().unwrap());
                continue;
            }
            tracing::info!("auto_update: pull OK para '{}', reiniciando...", name);
            match docker
                .restart_container(&cid, None::<RestartContainerOptions>)
                .await
            {
                Ok(_) => {
                    tracing::info!("auto_update: contenedor '{}' reiniciado correctamente", name);
                    let _ = notif_tx.send(NotifEvent {
                        container: name.clone(),
                        status: "🤖 auto-updated".into(),
                        timestamp: crate::timezone::now_time_formatted(),
                    });
                    notify_all(&settings, &name, "🤖 auto-actualizado").await;
                    {
                        let obj = db_pool.get().await.unwrap();
                        let _ = db::update_container_has_update(&obj.lock().unwrap(), &name, false);
                    }
                    let entry = UpdateHistoryEntry {
                        container: name.clone(),
                        image: image.clone(),
                        old_digest: String::new(),
                        new_digest: String::new(),
                        timestamp: crate::timezone::now_formatted(),
                        status: "auto-update".into(),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    };
                    let mut hist = update_history.lock().await;
                    hist.push(entry);
                    let obj = db_pool.get().await.unwrap();
                    let _ = db::append_update_history(&obj.lock().unwrap(), hist.last().unwrap());
                }
                Err(e) => {
                    tracing::warn!("auto_update: restart falló para '{}': {}", name, e);
                    let entry = UpdateHistoryEntry {
                        container: name.clone(),
                        image: image.clone(),
                        old_digest: String::new(),
                        new_digest: String::new(),
                        timestamp: crate::timezone::now_formatted(),
                        status: "auto-update-restart-error".into(),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    };
                    let mut hist = update_history.lock().await;
                    hist.push(entry);
                    let obj = db_pool.get().await.unwrap();
                    let _ = db::append_update_history(&obj.lock().unwrap(), hist.last().unwrap());
                }
            }

            // Limpiar imágenes viejas si la política lo indica
            if policy.cleanup_old_image {
                tracing::info!("auto_update: policy.cleanup_old_image=true, pruneando imágenes dangling para '{}'", name);
                let result = prune_dangling_images(&docker).await;
                log_prune_result("auto_update", &result);
            }
        }
    }
}

/// Verify a container is running after a restart
pub async fn verify_container_healthy(docker: &Docker, name: &str) -> bool {
    tokio::time::sleep(Duration::from_secs(3)).await;
    match find_container_by_name(docker, name).await {
        Ok(c) => c.state.as_deref() == Some("running"),
        Err(_) => false,
    }
}

/// Tag current image as backup for rollback: image:tag → image:rollback-{ts}
pub async fn tag_backup_image(docker: &Docker, image: &str) -> Option<(String, String, String)> {
    let ts = crate::timezone::now().format("%Y%m%d%H%M%S").to_string();
    if let Some((base, original_tag)) = image.rsplit_once(':') {
        let backup_full = format!("{}:rollback-{}", base, ts);
        let opts = bollard::image::TagImageOptions {
            repo: base.to_string(),
            tag: format!("rollback-{}", ts),
        };
        if docker.tag_image(image, Some(opts)).await.is_ok() {
            return Some((backup_full, base.to_string(), original_tag.to_string()));
        }
    }
    None
}

/// Rollback: restore backup tag, restart container, remove the new image
pub async fn rollback_container(
    docker: &Docker,
    cid: &str,
    base: &str,
    original_tag: &str,
    backup_full: &str,
    new_image: &str,
) {
    tracing::warn!("Rollback: restoring backup for {}", new_image);
    let restore_opts = bollard::image::TagImageOptions {
        repo: base.to_string(),
        tag: original_tag.to_string(),
    };
    let _ = docker.tag_image(backup_full, Some(restore_opts)).await;
    let _ = docker
        .restart_container(cid, None::<RestartContainerOptions>)
        .await;
    let _ = docker
        .remove_image(new_image, None::<bollard::image::RemoveImageOptions>, None)
        .await;
}
