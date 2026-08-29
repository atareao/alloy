use axum::{
    extract::{Path, State},
    response::Json,
};
use bollard::{
    container::{
        CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
        RemoveContainerOptions, RenameContainerOptions, RestartContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    image::{PruneImagesOptions, RemoveImageOptions, TagImageOptions},
    Docker,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::containers::{fetch_containers, find_container_by_name, pull_image};
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::check_remote_digest;
use crate::workers::resolve_compose_file;
use bollard::models::ImagePruneResponse;

/// Recreate a container by stopping, backing up, inspecting, creating with new image,
/// starting, and cleaning up the backup. On failure, attempts rollback.
pub(crate) async fn recreate_container(
    docker: &Docker,
    name: &str,
    cid: &str,
    image_full: &str,
) -> Result<(), String> {
    // 1. Stop the old container
    docker
        .stop_container(cid, None::<StopContainerOptions>)
        .await
        .map_err(|e| format!("stop failed: {}", e))?;

    // 2. Rename old container as backup
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let backup_name = format!("{}-old-{}", name, ts);
    docker
        .rename_container(
            cid,
            RenameContainerOptions {
                name: backup_name.clone(),
            },
        )
        .await
        .map_err(|e| format!("rename failed: {}", e))?;

    // 3. Inspect old container to get config
    let inspect = docker
        .inspect_container(&backup_name, None::<InspectContainerOptions>)
        .await
        .map_err(|e| format!("inspect failed: {}", e))?;

    // 4. Build new config from old one, modifying the image reference
    let mut config = inspect.config.unwrap_or_default();
    config.image = Some(image_full.to_string());

    // 5. Create new container with same name
    let create_opts = CreateContainerOptions {
        name: name.to_string(),
        platform: None,
    };
    docker
        .create_container(Some(create_opts), config.into())
        .await
        .map_err(|e| format!("create failed: {}", e))?;

    // 6. Start new container
    if let Err(e) = docker
        .start_container::<String>(name, None::<StartContainerOptions<String>>)
        .await
    {
        // If start fails, attempt rollback: remove the new container,
        // restore backup name, restart old container
        tracing::warn!("recreate_container: start failed for '{}', attempting rollback: {}", name, e);
        let _ = docker.remove_container(name, None::<RemoveContainerOptions>).await;
        let _ = docker.rename_container(
            &backup_name,
            RenameContainerOptions {
                name: name.to_string(),
            },
        ).await;
        let _ = docker.start_container::<String>(name, None::<StartContainerOptions<String>>).await;
        return Err(format!("start failed, rollback attempted: {}", e));
    }

    // 7. Remove old backup container
    let _ = docker
        .remove_container(&backup_name, None::<RemoveContainerOptions>)
        .await;

    Ok(())
}

struct PendingUpdate {
    name: String,
    image_full: String,
    cid: String,
    image_id: String,
    remote_digest: Option<String>,
    compose_project: Option<String>,
}

pub async fn update_container_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
    Path(name): Path<String>,
) -> Result<Json<UpdateProgress>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image = container.image.as_deref().unwrap_or("");
    let cid = container.id.as_deref().unwrap_or("");

    if image.is_empty() {
        return Err(AppError::BadRequest("container has no image".into()));
    }

    // Verificar digest remoto antes de hacer pull
    let image_id = container.image_id.as_deref().unwrap_or("").to_string();
    let (needs_pull, remote_digest) = match crate::updates::digest::check_remote_digest(image).await
    {
        Ok((digest, _)) => {
            if !image_id.is_empty() {
                let local_short = crate::updates::digest::short_digest(&image_id);
                let remote_short = crate::updates::digest::short_digest(&digest);
                (local_short != remote_short, digest)
            } else {
                (true, digest) // sin image_id local, asumimos que necesita pull
            }
        }
        Err(e) => {
            tracing::warn!(
                "update_container [{}]: error verificando digest remoto: {}, NO se hará pull",
                name,
                e
            );
            return Err(AppError::Internal(format!(
                "cannot check remote digest: {}",
                e
            )));
        }
    };

    if !needs_pull {
        let _ = update_tx.send(UpdateProgress {
            container: name.clone(),
            status: "✅ ya actualizado".into(),
            done: true,
            error: None,
        });
        return Ok(Json(UpdateProgress {
            container: name,
            status: "already-up-to-date".into(),
            done: true,
            error: None,
        }));
    }

    let _ = update_tx.send(UpdateProgress {
        container: name.clone(),
        status: format!("Pulling {}...", image),
        done: false,
        error: None,
    });
    let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
    let start_time = std::time::Instant::now();
    tracing::info!(
        "update_container_h: descargando '{}' (timeout: {}s)",
        image,
        pull_timeout
    );
    if !pull_image(&docker, image, pull_timeout).await {
        let _ = update_tx.send(UpdateProgress {
            container: name.clone(),
            status: "Error".into(),
            done: true,
            error: Some("pull failed".into()),
        });
        let entry = UpdateHistoryEntry {
            container: name.clone(),
            image: image.to_string(),
            old_digest: image_id.clone(),
            new_digest: remote_digest.clone(),
            timestamp: crate::timezone::now_formatted(),
            status: "error".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
        };
        let mut hist = update_history.lock().await;
        hist.push(entry);
        let conn = db_pool.get().await.unwrap();
        let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
        drop(conn);
        return Err(AppError::Internal("pull failed".into()));
    }
    let _ = update_tx.send(UpdateProgress {
        container: name.clone(),
        status: "Restarting...".into(),
        done: false,
        error: None,
    });
    match recreate_container(&docker, &name, cid, image).await
    {
        Ok(_) => {
            tracing::info!("update_container_h: '{}' reiniciado correctamente", name);
            let _ = update_tx.send(UpdateProgress {
                container: name.clone(),
                status: "✅ Restarted".into(),
                done: true,
                error: None,
            });
            let ts = crate::timezone::now_time_formatted();
            let _ = notif_tx.send(NotifEvent {
                container: name.clone(),
                status: "updated ✅".into(),
                timestamp: ts,
            });
            notify_all(&settings, &name, "✅ actualizado y reiniciado").await;
            crate::containers::remove_old_image(&docker, &image_id).await;
            {
                let conn = db_pool.get().await.unwrap();
                let _ = db::update_container_has_update(&conn.lock().unwrap(), &name, false);
            }
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.to_string(),
                old_digest: image_id.clone(),
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "success".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            Ok(Json(UpdateProgress {
                container: name,
                status: "ok".into(),
                done: true,
                error: None,
            }))
        }
        Err(e) => {
            tracing::error!("update_container_h: error al reiniciar '{}': {}", name, e);
            let _ = update_tx.send(UpdateProgress {
                container: name.clone(),
                status: "Error".into(),
                done: true,
                error: Some(e.to_string()),
            });
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.to_string(),
                old_digest: image_id.clone(),
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            Err(AppError::Docker(format!("restart: {}", e)))
        }
    }
}

pub async fn update_all_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
) -> Json<Vec<UpdateProgress>> {
    let mut results = vec![];
    for (name, image, cid, image_id) in crate::workers::docker_list_running(&docker).await {
        // Verificar digest remoto antes de hacer pull
        let (needs_pull, remote_digest) =
            match crate::updates::digest::check_remote_digest(&image).await {
                Ok((digest, _)) => {
                    let has_update = image_id.as_ref().is_none_or(|local_digest| {
                        let local_short = crate::updates::digest::short_digest(local_digest);
                        let remote_short = crate::updates::digest::short_digest(&digest);
                        local_short != remote_short
                    });
                    (has_update, digest)
                }
                Err(e) => {
                    tracing::warn!(
                        "update_all [{}]: error verificando digest remoto: {}, NO se hará pull",
                        name,
                        e
                    );
                    results.push(UpdateProgress {
                        container: name.clone(),
                        status: "error".into(),
                        done: true,
                        error: Some(format!("digest check failed: {}", e)),
                    });
                    continue;
                }
            };

        if !needs_pull {
            results.push(UpdateProgress {
                container: name.clone(),
                status: "✅ ya actualizado".into(),
                done: true,
                error: None,
            });
            continue;
        }

        let old_digest = image_id.as_deref().unwrap_or("").to_string();
        let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
        let start_time = std::time::Instant::now();
        tracing::info!(
            "update_all_h: descargando '{}' (imagen: {}, timeout: {}s)",
            name,
            image,
            pull_timeout
        );
        if !pull_image(&docker, &image, pull_timeout).await {
            tracing::error!("update_all_h: pull FALLÓ para '{}'", name);
            results.push(UpdateProgress {
                container: name.clone(),
                status: "error".into(),
                done: true,
                error: Some("pull failed".into()),
            });
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.clone(),
                old_digest,
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            continue;
        }
        tracing::info!("update_all_h: pull OK para '{}', reiniciando...", name);
        match recreate_container(&docker, &name, &cid, &image).await
        {
            Ok(_) => {
                tracing::info!(
                    "update_all_h: contenedor '{}' recreado correctamente",
                    name
                );
                let ts = crate::timezone::now_time_formatted();
                let _ = notif_tx.send(NotifEvent {
                    container: name.clone(),
                    status: "updated ✅".into(),
                    timestamp: ts,
                });
                notify_all(&settings, &name, "✅ actualizado").await;
                crate::containers::remove_old_image(&docker, &old_digest).await;
                {
                    let conn = db_pool.get().await.unwrap();
                    let _ = db::update_container_has_update(&conn.lock().unwrap(), &name, false);
                }
                results.push(UpdateProgress {
                    container: name.clone(),
                    status: "ok".into(),
                    done: true,
                    error: None,
                });
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest,
                    new_digest: remote_digest.clone(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "success".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            }
            Err(e) => {
                tracing::error!("update_all_h: error al reiniciar '{}': {}", name, e);
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest: old_digest.clone(),
                    new_digest: remote_digest.clone(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());

                results.push(UpdateProgress {
                    container: name,
                    status: "error".into(),
                    done: true,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    Json(results)
}

pub async fn check_update_h(
    State(docker): State<Docker>,
    State(db_pool): State<db::DbPool>,
    Path(name): Path<String>,
) -> Result<Json<VersionCompare>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image_full = container.image.as_deref().unwrap_or("");
    tracing::info!("check_update [{}]: imagen={}", name, image_full);
    let (has_update, local_tag, remote_digest, remote_tag, error) = if image_full.is_empty() {
        tracing::warn!("check_update [{}]: contenedor sin imagen", name);
        (None, "unknown".into(), None, None, Some("no image".into()))
    } else {
        let local_tag = crate::updates::digest::parse_image_ref(image_full).tag;
        let (remote_digest, remote_tag, error) = match check_remote_digest(image_full).await {
            Ok((digest, tag)) => (Some(digest), Some(tag), None),
            Err(e) => {
                tracing::warn!(
                    "check_update [{}]: error obteniendo digest remoto: {}",
                    name,
                    e
                );
                (None, None, Some(e))
            }
        };
        let has_update = match (&container.image_id, &remote_digest) {
            (Some(local_digest), Some(remote_digest)) => {
                let local_short = crate::updates::digest::short_digest(local_digest);
                let remote_short = crate::updates::digest::short_digest(remote_digest);
                let result = local_short != remote_short;
                tracing::info!(
                    "check_update [{}]: local={} remote={} has_update={}",
                    name,
                    local_short,
                    remote_short,
                    result
                );
                Some(result)
            }
            _ => None,
        };
        (has_update, local_tag, remote_digest, remote_tag, error)
    };
    // Persist has_update to database
    if let Some(hu) = has_update {
        let conn = db_pool.get().await.unwrap();
        let _ = db::update_container_has_update(&conn.lock().unwrap(), &name, hu);
        drop(conn);
    }
    let local_digest = container
        .image_id
        .as_ref()
        .map(|d| crate::updates::digest::short_digest(d));
    Ok(Json(VersionCompare {
        local_tag,
        remote_tag,
        has_update,
        local_digest,
        remote_digest: remote_digest.map(|d| crate::updates::digest::short_digest(&d)),
        changelog_url: None,
        error,
    }))
}

/// Check all containers sequentially, applying policies inline.
/// Returns the container list with has_update flags updated.
/// This replaces the old parallel check + background apply pattern to avoid
/// Docker Hub rate limiting (429 errors).
#[allow(clippy::too_many_arguments)]
async fn check_and_apply_all(
    docker: &Docker,
    db_pool: &DbPool,
    tx: &broadcast::Sender<StateEvent>,
    update_tx: &broadcast::Sender<UpdateProgress>,
    settings: &Arc<Mutex<Settings>>,
    notif_tx: &broadcast::Sender<NotifEvent>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
) -> Vec<ContainerInfo> {
    let mut containers = fetch_containers(docker, &None, db_pool).await;
    let raw_containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
    tracing::info!("check_and_apply_all: verificando {} contenedores", containers.len());

    let check_interval = {
        let s = settings.lock().await;
        s.check_interval_ms.unwrap_or(2000)
    };

    // Pre-build a name→(&ContainerSummary, image_id) map from raw containers
    let raw_map: HashMap<String, (&bollard::models::ContainerSummary, String)> = raw_containers
        .iter()
        .filter_map(|ct| {
            let name = ct.names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| crate::models::strip_name(n))?;
            let image_id = ct.image_id.as_deref().unwrap_or("").to_string();
            Some((name, (ct, image_id)))
        })
        .collect();

    let mut any_success = false;

    for c in &mut containers {
        let name = c.name.clone();
        let image_full = if c.image_tag.is_empty() {
            c.image.clone()
        } else {
            format!("{}:{}", c.image, c.image_tag)
        };
        let last_remote_digest = c.last_remote_digest.clone();

        if image_full.is_empty() {
            tracing::warn!("check_and_apply_all [{}]: sin imagen, omitiendo", name);
            continue;
        }

        // Get image_id and cid from raw map
        let (raw, image_id) = match raw_map.get(name.as_str()) {
            Some((ct, id)) => (*ct, id.clone()),
            None => {
                tracing::warn!("check_and_apply_all [{}]: no encontrado en raw_containers, omitiendo", name);
                continue;
            }
        };

        tracing::info!("check_and_apply_all [{}]: verificando imagen {}", name, image_full);

        match check_remote_digest(&image_full).await {
            Ok((remote_digest, _)) => {
                let local_ref = if !last_remote_digest.is_empty() {
                    &last_remote_digest
                } else {
                    &image_id
                };
                let local_short = if local_ref.is_empty() {
                    String::new()
                } else {
                    crate::updates::digest::short_digest(local_ref)
                };
                let remote_short = crate::updates::digest::short_digest(&remote_digest);
                let has_update = !local_ref.is_empty() && local_short != remote_short;

                tracing::info!(
                    "check_and_apply_all [{}]: local={} remote={} has_update={} (using={})",
                    name, local_short, remote_short, has_update,
                    if !last_remote_digest.is_empty() { "last_remote_digest" } else { "image_id" }
                );

                // Update has_update in container and DB
                c.has_update = has_update;
                {
                    let conn = db_pool.get().await.unwrap();
                    let conn_lock = conn.lock().unwrap();
                    if let Err(e) = db::update_container_has_update(&conn_lock, &name, has_update) {
                        tracing::error!("check_and_apply_all: error updating has_update for '{}': {}", name, e);
                    }
                    if let Err(e) = db::update_container_last_remote_digest(&conn_lock, &name, &remote_digest) {
                        tracing::error!("check_and_apply_all: error storing last_remote_digest for '{}': {}", name, e);
                    }
                    drop(conn_lock);
                }

                // If has_update and running → apply policy inline
                if has_update && c.state == "running" {
                    let cid = raw.id.as_deref().unwrap_or("").to_string();
                    let compose_project = raw.labels
                        .as_ref()
                        .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                        .cloned();
                    let pending = PendingUpdate {
                        name: name.clone(),
                        image_full: image_full.clone(),
                        cid,
                        image_id,
                        remote_digest: Some(remote_digest),
                        compose_project,
                    };
                    // Resolve policy for this single container
                    let policies = update_policies.lock().await.clone();
                    let policy_map: HashMap<String, UpdatePolicy> = policies
                        .into_iter()
                        .map(|p| (p.container.clone(), p))
                        .collect();
                    let (default_action, default_cleanup, default_rollback) = {
                        let s = settings.lock().await;
                        (
                            s.default_update_action.clone().unwrap_or_else(|| "pull-restart".into()),
                            s.default_cleanup_old_image.unwrap_or(false),
                            s.default_rollback_on_failure.unwrap_or(false),
                        )
                    };
                    let policy = policy_map.get(&name).cloned().unwrap_or(UpdatePolicy {
                        container: name.clone(),
                        action: default_action.parse().unwrap_or(UpdateAction::PullRestart),
                        cleanup_old_image: default_cleanup,
                        rollback_on_failure: default_rollback,
                        notify_events: false,
                    });
                    apply_single_policy(
                        docker, settings, update_tx, notif_tx, update_history,
                        db_pool, tx, &pending, &policy,
                        &mut any_success,
                    ).await;
                }
            }
            Err(e) => {
                if e.contains("429") {
                    tracing::warn!("check_and_apply_all [{}]: rate limit (429) fetching digest, saltando", name);
                } else if e.contains("404") {
                    tracing::warn!("check_and_apply_all [{}]: imagen remota no encontrada (404): {}", name, image_full);
                } else if e.contains("403") {
                    tracing::warn!("check_and_apply_all [{}]: acceso denegado (403) a: {}", name, image_full);
                } else {
                    tracing::warn!("check_and_apply_all [{}]: error consultando digest remoto: {}", name, e);
                }
            }
        }

        // Sleep between checks to avoid hammering registries
        if check_interval > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(check_interval)).await;
        }
    }

    // Update timestamps
    {
        let conn = db_pool.get().await.unwrap();
        let mut s = settings.lock().await;
        let cron = s.update_check_cron.clone().unwrap_or_else(|| "0 0 * * *".into());
        let last_check = crate::timezone::now_formatted();
        let next_check = crate::timezone::next_cron_time(&cron).unwrap_or_default();
        for c in &containers {
            let _ = db::update_container_check_times(&conn.lock().unwrap(), &c.name, &last_check, &next_check);
        }
        s.update_check_last_run_at = Some(last_check);
        let _ = db::save_settings(&conn.lock().unwrap(), &s);
    }

    // Send StateEvent if any success
    if any_success {
        tracing::info!("check_and_apply_all: enviando StateEvent para refrescar frontend");
        let containers = fetch_containers(docker, &None, db_pool).await;
        let _ = tx.send(StateEvent { containers });
    }

    containers
}

#[allow(clippy::too_many_arguments)]
pub async fn check_all_h(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(update_policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
) -> Json<Vec<ContainerInfo>> {
    let containers = check_and_apply_all(
        &docker, &db_pool, &tx, &update_tx, &settings,
        &notif_tx, &update_history, &update_policies,
    ).await;
    Json(containers)
}

/// Apply a single container's update policy inline.
/// Contains the same logic as the per-container loop body in `apply_policies_background`.
#[allow(clippy::too_many_arguments)]
async fn apply_single_policy(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    update_tx: &broadcast::Sender<UpdateProgress>,
    notif_tx: &broadcast::Sender<NotifEvent>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: &DbPool,
    _tx: &broadcast::Sender<StateEvent>,
    p: &PendingUpdate,
    policy: &UpdatePolicy,
    any_success: &mut bool,
) {
    if policy.action == UpdateAction::None {
        tracing::warn!(
            "apply_single_policy: política None para '{}', saltando",
            p.name
        );
        let _ = update_tx.send(UpdateProgress {
            container: p.name.clone(),
            status: "⏭️ política: no hacer nada".into(),
            done: true,
            error: None,
        });
        return;
    }

    tracing::info!(
        "apply_single_policy: procesando '{}' con acción {:?} (imagen: {}, cleanup_old_image: {})",
        p.name,
        policy.action,
        p.image_full,
        policy.cleanup_old_image
    );

    let _ = update_tx.send(UpdateProgress {
        container: p.name.clone(),
        status: format!("🔄 actualizando {}...", p.name),
        done: false,
        error: None,
    });

    let start_time = std::time::Instant::now();
    let mut success = false;
    let pull_timeout = {
        let s = settings.lock().await;
        s.pull_timeout_secs.unwrap_or(600)
    };

    match policy.action {
        UpdateAction::Pull => {
            tracing::info!(
                "apply_single_policy: Pull '{}' desde '{}'",
                p.name,
                p.image_full
            );
            if pull_image(docker, &p.image_full, pull_timeout).await {
                tracing::info!("apply_single_policy: Pull OK '{}'", p.name);
                let _ = update_tx.send(UpdateProgress {
                    container: p.name.clone(),
                    status: "✅ pulled".into(),
                    done: true,
                    error: None,
                });
                success = true;
            } else {
                tracing::error!("apply_single_policy: Pull FALLÓ '{}'", p.name);
                let _ = update_tx.send(UpdateProgress {
                    container: p.name.clone(),
                    status: "❌ pull falló".into(),
                    done: true,
                    error: Some("pull_image returned false".into()),
                });
                let entry = UpdateHistoryEntry {
                    container: p.name.clone(),
                    image: p.image_full.clone(),
                    old_digest: p.image_id.clone(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "apply-policy-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            }
        }
        UpdateAction::PullRestart => {
            tracing::info!(
                "apply_single_policy: PullRestart '{}' desde '{}'",
                p.name,
                p.image_full
            );
            let backup = if policy.rollback_on_failure {
                tag_backup_image(docker, &p.image_full).await
            } else {
                None
            };
            if pull_image(docker, &p.image_full, pull_timeout).await {
                tracing::info!(
                    "apply_single_policy: Pull OK, reiniciando contenedor '{}' (cid: {})",
                    p.name,
                    p.cid
                );
                let _ = update_tx.send(UpdateProgress {
                    container: p.name.clone(),
                    status: "🔄 reiniciando contenedor...".into(),
                    done: false,
                    error: None,
                });
                match recreate_container(docker, &p.name, &p.cid, &p.image_full).await {
                    Ok(_) => {
                        tracing::info!(
                            "apply_single_policy: contenedor '{}' recreado correctamente",
                            p.name
                        );
                        if policy.rollback_on_failure
                            && !verify_container_healthy(docker, &p.name).await
                        {
                            tracing::warn!("apply_single_policy: rollback '{}'", p.name);
                            if let Some((backup_full, base, orig_tag)) = backup {
                                rollback_container(
                                    docker,
                                    &p.cid,
                                    &base,
                                    &orig_tag,
                                    &backup_full,
                                    &p.image_full,
                                )
                                .await;
                            }
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "⚠️ rollback aplicado".into(),
                                done: true,
                                error: Some("container no healthy".into()),
                            });
                        } else {
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "✅ actualizado + reiniciado".into(),
                                done: true,
                                error: None,
                            });
                            success = true;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "apply_single_policy: error al reiniciar '{}': {}",
                            p.name,
                            e
                        );
                        let _ = update_tx.send(UpdateProgress {
                            container: p.name.clone(),
                            status: "❌ error al reiniciar".into(),
                            done: true,
                            error: Some(e.to_string()),
                        });
                        let entry = UpdateHistoryEntry {
                            container: p.name.clone(),
                            image: p.image_full.clone(),
                            old_digest: p.image_id.clone(),
                            new_digest: String::new(),
                            timestamp: crate::timezone::now_formatted(),
                            status: "apply-policy-restart-error".into(),
                            duration_ms: start_time.elapsed().as_millis() as u64,
                        };
                        let mut hist = update_history.lock().await;
                        hist.push(entry);
                        let conn = db_pool.get().await.unwrap();
                        let _ = db::append_update_history(
                            &conn.lock().unwrap(),
                            hist.last().unwrap(),
                        );
                    }
                }
            } else {
                tracing::error!("apply_single_policy: Pull FALLÓ '{}'", p.name);
                let _ = update_tx.send(UpdateProgress {
                    container: p.name.clone(),
                    status: "❌ pull falló".into(),
                    done: true,
                    error: Some("pull_image returned false".into()),
                });
                let entry = UpdateHistoryEntry {
                    container: p.name.clone(),
                    image: p.image_full.clone(),
                    old_digest: p.image_id.clone(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "apply-policy-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            }
        }
        UpdateAction::PullRestartStack => {
            if let Some(ref project) = p.compose_project {
                let compose_file = resolve_compose_file(docker, project).await;
                if let Some(ref file) = compose_file {
                    tracing::info!(
                        "apply_single_policy: PullRestartStack '{}' (proyecto: {})",
                        p.name,
                        project
                    );
                    let _ = update_tx.send(UpdateProgress {
                        container: p.name.clone(),
                        status: format!("📥 Pulling stack '{}'...", project),
                        done: false,
                        error: None,
                    });
                    let pull = tokio::process::Command::new("docker")
                        .args(["compose", "-f", file, "pull"])
                        .output()
                        .await;
                    match pull {
                        Ok(output) if output.status.success() => {
                            tracing::info!(
                                "apply_single_policy: Pull stack OK, recreando '{}'",
                                project
                            );
                            let _ = tokio::process::Command::new("docker")
                                .args(["compose", "-f", file, "up", "-d"])
                                .output()
                                .await;
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "✅ stack updated".into(),
                                done: true,
                                error: None,
                            });
                            success = true;
                        }
                        Ok(output) => {
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            tracing::error!(
                                "apply_single_policy: Pull stack FALLÓ '{}': {}",
                                project,
                                stderr
                            );
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "❌ pull falló".into(),
                                done: true,
                                error: Some(stderr),
                            });
                        }
                        Err(e) => {
                            tracing::error!(
                                "apply_single_policy: error al ejecutar docker compose: {}",
                                e
                            );
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "❌ error".into(),
                                done: true,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    tracing::error!(
                        "apply_single_policy: compose file no encontrado para '{}'",
                        project
                    );
                    let _ = update_tx.send(UpdateProgress {
                        container: p.name.clone(),
                        status: "❌ compose file no encontrado".into(),
                        done: true,
                        error: Some("cannot resolve compose file".into()),
                    });
                }
            } else {
                let _ = update_tx.send(UpdateProgress {
                    container: p.name.clone(),
                    status: "❌ no es stack".into(),
                    done: true,
                    error: Some("container has no compose project label".into()),
                });
            }
        }
        _ => {
            let _ = update_tx.send(UpdateProgress {
                container: p.name.clone(),
                status: "⏭️ acción desconocida".into(),
                done: true,
                error: None,
            });
        }
    }

    if success {
        tracing::info!("apply_single_policy: éxito '{}'", p.name);
        let _ = notif_tx.send(NotifEvent {
            container: p.name.clone(),
            status: "updated ✅".into(),
            timestamp: crate::timezone::now_time_formatted(),
        });
        notify_all(settings, &p.name, "✅ actualizado y reiniciado").await;
        // Store remote digest to prevent re-detection on next cycle.
        // Use the digest already obtained in check_all_h if available,
        // to avoid a redundant registry query that may hit rate limits.
        let new_digest = if let Some(d) = &p.remote_digest {
            d.clone()
        } else {
            check_remote_digest_on_image(&p.image_full).await
        };
        {
            let conn = db_pool.get().await.unwrap();
            if let Err(e) = db::update_container_has_update(&conn.lock().unwrap(), &p.name, false) {
                tracing::error!("apply_single_policy: error clearing has_update for '{}': {}", p.name, e);
            }
            if !new_digest.is_empty() {
                if let Err(e) = db::update_container_last_remote_digest(&conn.lock().unwrap(), &p.name, &new_digest) {
                    tracing::error!("apply_single_policy: error storing last_remote_digest for '{}': {}", p.name, e);
                }
            } else {
                tracing::warn!("apply_single_policy: no se pudo obtener digest remoto para '{}', last_remote_digest no actualizado", p.name);
            }
        }
        let entry = UpdateHistoryEntry {
            container: p.name.clone(),
            image: p.image_full.clone(),
            old_digest: p.image_id.clone(),
            new_digest: new_digest.clone(),
            timestamp: crate::timezone::now_formatted(),
            status: "success".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
        };
        let mut hist = update_history.lock().await;
        hist.push(entry);
        let conn = db_pool.get().await.unwrap();
        let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());

        *any_success = true;

        // Limpiar imágenes dangling si la política lo indica
        if policy.cleanup_old_image {
            tracing::info!(
                "apply_single_policy: policy.cleanup_old_image=true, haciendo prune de imágenes dangling para '{}'",
                p.name
            );
            let result = prune_dangling_images(docker).await;
            log_prune_result("policy.cleanup_old_image", &result);
        }
    } else {
        tracing::warn!(
            "apply_single_policy: fallo/no-hubo-éxito '{}'",
            p.name
        );
    }
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
async fn apply_policies_background(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    tx: &broadcast::Sender<StateEvent>,
    update_tx: &broadcast::Sender<UpdateProgress>,
    notif_tx: &broadcast::Sender<NotifEvent>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
    db_pool: &DbPool,
    pending: &[PendingUpdate],
) {
    tracing::info!(
        "apply_policies_background: iniciando con {} pendientes: {:?}",
        pending.len(),
        pending.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
    let policies = update_policies.lock().await.clone();
    let policy_map: HashMap<String, UpdatePolicy> = policies
        .into_iter()
        .map(|p| (p.container.clone(), p))
        .collect();
    let (default_action, default_cleanup, default_rollback) = {
        let s = settings.lock().await;
        (
            s.default_update_action
                .clone()
                .unwrap_or_else(|| "pull-restart".into()),
            s.default_cleanup_old_image.unwrap_or(false),
            s.default_rollback_on_failure.unwrap_or(false),
        )
    };

    let mut any_success = false;

    for p in pending {
        let policy = match policy_map.get(&p.name) {
            Some(pol) => pol.clone(),
            None => UpdatePolicy {
                container: p.name.clone(),
                action: default_action.parse().unwrap_or(UpdateAction::PullRestart),
                cleanup_old_image: default_cleanup,
                rollback_on_failure: default_rollback,
                notify_events: false,
            },
        };
        apply_single_policy(
            docker, settings, update_tx, notif_tx, update_history,
            db_pool, tx, p, &policy, &mut any_success,
        ).await;
    }

    // Safety net: limpiar dangling images que hayan podido quedar
    tracing::info!("apply_policies_background: completado, iniciando prune de imágenes dangling");
    let result = prune_dangling_images(docker).await;
    log_prune_result("safety-net", &result);

    // Enviar StateEvent para que el frontend refresque inmediatamente
    if any_success {
        tracing::info!("apply_policies_background: enviando StateEvent para refrescar frontend");
        let containers = fetch_containers(docker, &None, db_pool).await;
        let _ = tx.send(StateEvent { containers });
    }
}

/// Prune dangling images (no tag, no container reference)
pub(crate) async fn prune_dangling_images(
    docker: &Docker,
) -> Result<ImagePruneResponse, bollard::errors::Error> {
    let mut filters = HashMap::new();
    filters.insert("dangling", vec!["true"]);
    let opts = PruneImagesOptions { filters };
    docker.prune_images(Some(opts)).await
}

/// Log the result of a prune operation
pub(crate) fn log_prune_result(
    context: &str,
    result: &Result<ImagePruneResponse, bollard::errors::Error>,
) {
    match result {
        Ok(resp) => {
            let deleted_count = resp.images_deleted.as_ref().map(|v| v.len()).unwrap_or(0);
            let reclaimed = resp.space_reclaimed.unwrap_or(0);
            if deleted_count > 0 {
                tracing::info!(
                    "prune_images [{}]: {} imágenes eliminadas, {} bytes liberados",
                    context,
                    deleted_count,
                    reclaimed
                );
            } else {
                tracing::info!(
                    "prune_images [{}]: no hay imágenes dangling para eliminar",
                    context
                );
            }
        }
        Err(e) => {
            tracing::warn!("prune_images [{}]: error: {}", context, e);
        }
    }
}

/// Verify a container is running after a restart
pub(crate) async fn verify_container_healthy(docker: &Docker, name: &str) -> bool {
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    match find_container_by_name(docker, name).await {
        Ok(c) => c.state.as_deref() == Some("running"),
        Err(_) => false,
    }
}

/// Tag current image as backup for rollback: image:tag → image:rollback-{ts}
pub(crate) async fn tag_backup_image(docker: &Docker, image: &str) -> Option<(String, String, String)> {
    let ts = crate::timezone::now().format("%Y%m%d%H%M%S").to_string();
    if let Some((base, original_tag)) = image.rsplit_once(':') {
        let backup_full = format!("{}:rollback-{}", base, ts);
        let opts = TagImageOptions {
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
pub(crate) async fn rollback_container(
    docker: &Docker,
    cid: &str,
    base: &str,
    original_tag: &str,
    backup_full: &str,
    new_image: &str,
) {
    tracing::warn!("Rollback: restoring backup for {}", new_image);
    let restore_opts = TagImageOptions {
        repo: base.to_string(),
        tag: original_tag.to_string(),
    };
    let _ = docker.tag_image(backup_full, Some(restore_opts)).await;
    let _ = docker
        .restart_container(cid, None::<RestartContainerOptions>)
        .await;
    let _ = docker
        .remove_image(new_image, None::<RemoveImageOptions>, None)
        .await;
}

/// Llama a `check_remote_digest` con la referencia de imagen completa.
/// Retorna el digest remoto o cadena vacía si falla la consulta.
async fn check_remote_digest_on_image(image_full: &str) -> String {
    match crate::updates::digest::check_remote_digest(image_full).await {
        Ok((digest, _)) => digest,
        Err(_) => String::new(),
    }
}
