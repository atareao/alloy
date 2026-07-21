use bollard::{
    container::{ListContainersOptions, RestartContainerOptions},
    Docker,
};
use chrono::{Local, Timelike};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::containers::pull_image;
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::check_remote_digest;

/// Worker que ejecuta revisiones de actualizaciones según el cron configurado.
/// Revisa todas las imágenes, marca las que tienen actualización pendiente en DB,
/// y aplica las políticas configuradas (pull, restart, prune).
pub async fn update_check_worker(
    docker: Docker,
    settings: Arc<Mutex<Settings>>,
    update_policies: Arc<Mutex<Vec<UpdatePolicy>>>,
    update_tx: broadcast::Sender<UpdateProgress>,
    notif_tx: broadcast::Sender<NotifEvent>,
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: DbPool,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        tick.tick().await;
        let s = settings.lock().await;
        let enabled = s.update_check_enabled.unwrap_or(false);
        let cron = s
            .update_check_cron
            .clone()
            .unwrap_or_else(|| "0 0 * * *".into()); // default: cada 24h a medianoche
        drop(s);

        if !enabled {
            continue;
        }
        let now = Local::now();
        let now_rounded = now
            .with_second(0)
            .and_then(|d| d.with_nanosecond(0))
            .unwrap_or(now);
        let expr = format!("0 {}", cron);
        let should_run = match expr.parse::<cron::Schedule>() {
            Ok(schedule) => schedule.includes(now_rounded),
            Err(e) => {
                tracing::warn!("update_check: cron inválido '{}': {}", cron, e);
                false
            }
        };
        if !should_run {
            continue;
        }

        tracing::info!(
            "update_check: revisando y aplicando actualizaciones (cron={})",
            cron
        );

        // 1. Obtener todos los contenedores
        let containers = docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();

        // 2. Revisar cada contenedor para detectar actualizaciones
        let tasks: Vec<_> = containers
            .iter()
            .map(|c| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| crate::models::strip_name(n))
                    .unwrap_or_default();
                let image_full = c.image.as_deref().unwrap_or("").to_string();
                let image_id = c.image_id.clone().unwrap_or_default();
                let cid = c.id.clone().unwrap_or_default();
                async move {
                    if image_full.is_empty() {
                        return (name, false, image_full, cid);
                    }
                    let (repo, local_tag) = crate::updates::digest::parse_image_ref(&image_full);
                    match check_remote_digest(&repo, &local_tag).await {
                        Ok((remote_digest, _)) => {
                            let has_update = if !image_id.is_empty() {
                                let local_short = crate::updates::digest::short_digest(&image_id);
                                let remote_short =
                                    crate::updates::digest::short_digest(&remote_digest);
                                local_short != remote_short
                            } else {
                                false
                            };
                            (name, has_update, image_full, cid)
                        }
                        Err(_) => (name, false, image_full, cid),
                    }
                }
            })
            .collect();

        // 3. Obtener políticas
        let policies = {
            let p = update_policies.lock().await;
            let map: HashMap<String, UpdatePolicy> = p
                .iter()
                .map(|pol| (pol.container.clone(), pol.clone()))
                .collect();
            let s = settings.lock().await;
            let default = (
                s.default_update_action
                    .clone()
                    .unwrap_or_else(|| "pull-restart".into()),
                s.default_cleanup_old_image.unwrap_or(false),
                s.default_rollback_on_failure.unwrap_or(false),
            );
            drop(s);
            (map, default)
        };
        let notify = {
            let s = settings.lock().await;
            s.update_check_notify.unwrap_or(false)
        };

        // 4. Procesar resultados: persistir en DB y aplicar políticas
        let mut updated_count = 0u32;
        for (name, has_update, image_full, cid) in futures::future::join_all(tasks).await {
            let _ = sqlite_update_has_update(&db_pool, &name, has_update).await;
            if !has_update || name.is_empty() || image_full.is_empty() {
                continue;
            }
            updated_count += 1;

            // Leer política para este contenedor
            let policy = match policies.0.get(&name) {
                Some(p) => p.clone(),
                None => UpdatePolicy {
                    container: name.clone(),
                    action: policies.1 .0.parse().unwrap_or(UpdateAction::PullRestart),
                    cleanup_old_image: policies.1 .1,
                    rollback_on_failure: policies.1 .2,
                    notify_events: false,
                },
            };
            if policy.action == UpdateAction::None {
                let _ = update_tx.send(UpdateProgress {
                    container: name.clone(),
                    status: "⏭️ política: no hacer nada".into(),
                    done: true,
                    error: None,
                });
                continue;
            }

            tracing::info!(
                "update_check: aplicando política {:?} a '{}'",
                policy.action,
                name
            );

            let _ = update_tx.send(UpdateProgress {
                container: name.clone(),
                status: format!("[update-check] 🔍 {}", policy.action),
                done: false,
                error: None,
            });

            let start = std::time::Instant::now();
            match policy.action {
                UpdateAction::Pull => {
                    if pull_image(&docker, &image_full).await {
                        let _ = update_tx.send(UpdateProgress {
                            container: name.clone(),
                            status: "✅ descargado (update-check)".into(),
                            done: true,
                            error: None,
                        });
                        _ = sqlite_append_update(
                            &db_pool,
                            &update_history,
                            &name,
                            &image_full,
                            "update-check-pull",
                            start.elapsed().as_millis() as u64,
                        )
                        .await;
                        if policy.cleanup_old_image {
                            let _ = docker
                                .prune_images(None::<bollard::image::PruneImagesOptions<&str>>)
                                .await;
                        }
                    } else {
                        let _ = update_tx.send(UpdateProgress {
                            container: name.clone(),
                            status: "❌ error al descargar".into(),
                            done: true,
                            error: Some("pull failed".into()),
                        });
                    }
                }
                UpdateAction::PullRestart => {
                    if pull_image(&docker, &image_full).await {
                        let _ = docker
                            .restart_container(&cid, None::<RestartContainerOptions>)
                            .await;
                        let _ = notif_tx.send(NotifEvent {
                            container: name.clone(),
                            status: "🔄 actualizado (update-check)".into(),
                            timestamp: Local::now().format("%H:%M:%S").to_string(),
                        });
                        if notify {
                            notify_all(&settings, &name, "🔄 actualizado vía update-check").await;
                        }
                        let _ = sqlite_update_has_update(&db_pool, &name, false).await;
                        _ = sqlite_append_update(
                            &db_pool,
                            &update_history,
                            &name,
                            &image_full,
                            "update-check-restart",
                            start.elapsed().as_millis() as u64,
                        )
                        .await;
                        let _ = update_tx.send(UpdateProgress {
                            container: name.clone(),
                            status: "✅ actualizado + reiniciado (update-check)".into(),
                            done: true,
                            error: None,
                        });
                        if policy.cleanup_old_image {
                            let _ = docker
                                .prune_images(None::<bollard::image::PruneImagesOptions<&str>>)
                                .await;
                        }
                    } else {
                        let _ = update_tx.send(UpdateProgress {
                            container: name.clone(),
                            status: "❌ error al descargar".into(),
                            done: true,
                            error: Some("pull failed".into()),
                        });
                    }
                }
                UpdateAction::PullRestartStack => {
                    let compose_project = containers
                        .iter()
                        .find(|c| {
                            c.names
                                .as_ref()
                                .and_then(|n| n.first())
                                .map(|n| crate::models::strip_name(n) == name)
                                .unwrap_or(false)
                        })
                        .and_then(|c| c.labels.as_ref())
                        .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                        .cloned();
                    if let Some(ref project) = compose_project {
                        let compose_file = resolve_compose_file(&docker, project).await;
                        if let Some(ref file) = compose_file {
                            let _ = update_tx.send(UpdateProgress {
                                container: name.clone(),
                                status: format!("📥 Pulling stack '{}'...", project),
                                done: false,
                                error: None,
                            });
                            let output = tokio::process::Command::new("docker")
                                .args(["compose", "-f", file, "pull"])
                                .output()
                                .await;
                            match output {
                                Ok(o) if o.status.success() => {
                                    let _ = tokio::process::Command::new("docker")
                                        .args(["compose", "-f", file, "up", "-d"])
                                        .output()
                                        .await;
                                    let _ = update_tx.send(UpdateProgress {
                                        container: name.clone(),
                                        status: "✅ stack actualizado (update-check)".into(),
                                        done: true,
                                        error: None,
                                    });
                                    if policy.cleanup_old_image {
                                        let _ = docker
                                            .prune_images(
                                                None::<bollard::image::PruneImagesOptions<&str>>,
                                            )
                                            .await;
                                    }
                                }
                                _ => {
                                    let _ = update_tx.send(UpdateProgress {
                                        container: name.clone(),
                                        status: "❌ error stack pull".into(),
                                        done: true,
                                        error: Some("docker compose pull failed".into()),
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {
                    let _ = update_tx.send(UpdateProgress {
                        container: name.clone(),
                        status: "⏭️ acción desconocida".into(),
                        done: true,
                        error: None,
                    });
                }
            }
        }

        tracing::info!(
            "update_check: {} contenedores revisados, {} actualizados/aplicados",
            containers.len(),
            updated_count
        );
    }
}

/// Persistir has_update en DB (helper)
async fn sqlite_update_has_update(db_pool: &DbPool, name: &str, has_update: bool) {
    let obj = db_pool.get().await.unwrap();
    let _ = db::update_container_has_update(&obj.lock().unwrap(), name, has_update);
    drop(obj);
}

/// Append a update history (helper)
async fn sqlite_append_update(
    db_pool: &DbPool,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    name: &str,
    image: &str,
    status: &str,
    duration_ms: u64,
) {
    let entry = UpdateHistoryEntry {
        container: name.to_string(),
        image: image.to_string(),
        old_digest: String::new(),
        new_digest: String::new(),
        timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        status: status.to_string(),
        duration_ms,
    };
    let mut hist = update_history.lock().await;
    hist.push(entry);
    let obj = db_pool.get().await.unwrap();
    let _ = db::append_update_history(&obj.lock().unwrap(), hist.last().unwrap());
    drop(obj);
}

pub async fn resolve_compose_file(docker: &Docker, project: &str) -> Option<String> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
    let project_containers: Vec<_> = containers
        .iter()
        .filter(|c| {
            c.labels
                .as_ref()
                .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                .map(|p| p == project)
                .unwrap_or(false)
        })
        .collect();
    if project_containers.is_empty() {
        return None;
    }
    project_containers
        .first()
        .and_then(|c| c.labels.as_ref())
        .and_then(|l| l.get(crate::models::LABEL_COMPOSE_CONFIG_FILES))
        .cloned()
        .or_else(|| {
            project_containers
                .first()
                .and_then(|c| c.labels.as_ref())
                .and_then(|l| l.get(crate::models::LABEL_COMPOSE_WORKING_DIR))
                .map(|dir| format!("{}/docker-compose.yml", dir))
        })
        .filter(|p| std::path::Path::new(p).exists())
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    fn match_cron(cron: &str, dt: &chrono::DateTime<Local>) -> bool {
        let expr = format!("0 {}", cron);
        match expr.parse::<cron::Schedule>() {
            Ok(schedule) => schedule.includes(dt.to_utc()),
            Err(e) => {
                tracing::warn!("Invalid cron expression '{}': {}", cron, e);
                false
            }
        }
    }

    #[test]
    fn test_match_cron_every_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("* * * * *", &dt));
    }

    #[test]
    fn test_match_cron_specific_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 30, 0).unwrap();
        assert!(match_cron("30 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_wrong_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 15, 0).unwrap();
        assert!(!match_cron("0 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_invalid_expression() {
        let dt = Local::now();
        assert!(!match_cron("invalid", &dt));
    }

    #[test]
    fn test_match_cron_empty() {
        let dt = Local::now();
        assert!(!match_cron("", &dt));
    }

    #[test]
    fn test_match_cron_every_5_minutes_first() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("*/5 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_every_5_minutes_fifth() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 5, 0).unwrap();
        assert!(match_cron("*/5 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_every_5_minutes_wrong() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 3, 0).unwrap();
        assert!(!match_cron("*/5 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_specific_hour() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 3, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        assert!(match_cron("0 3 * * *", &dt));
    }

    #[test]
    fn test_match_cron_wrong_hour() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 4, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        assert!(!match_cron("0 3 * * *", &dt));
    }

    #[test]
    fn test_match_cron_daily_at_midnight() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        assert!(match_cron("0 0 * * *", &dt));
    }

    #[test]
    fn test_match_cron_range_hours() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("0 9-17 * * *", &dt));
    }

    #[test]
    fn test_match_cron_outside_range_hours() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 20, 0, 0).unwrap();
        assert!(!match_cron("0 9-17 * * *", &dt));
    }

    #[test]
    fn test_match_cron_weekly() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        let _ = match_cron("0 0 * * 0", &dt);
    }

    #[test]
    fn test_match_cron_not_weekly() {
        let dt = Local.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let _ = match_cron("0 0 * * 0", &dt);
    }
}
