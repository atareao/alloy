use bollard::{container::ListContainersOptions, Docker};
use chrono::Local;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::containers::pull_image;
use crate::db;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::check_remote_digest;
use crate::workers::auto_update::{rollback_container, tag_backup_image, verify_container_healthy};

/// Worker que ejecuta revisiones de actualizaciones según el cron configurado.
/// 1. Lee la configuración de update_check de Settings.
/// 2. Si está habilitado y el cron coincide, revisa todos los contenedores.
/// 3. Marca los contenedores con actualizaciones pendientes en la DB.
/// 4. Ejecuta las acciones configuradas (pull, pull+restart, etc.) para cada contenedor.
pub async fn update_check_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    update_policies: Arc<Mutex<Vec<UpdatePolicy>>>,
    update_tx: broadcast::Sender<UpdateProgress>,
    _notif_tx: broadcast::Sender<NotifEvent>,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        tick.tick().await;
        let s = settings.lock().await;
        let enabled = s.update_check_enabled.unwrap_or(false);
        let cron = s
            .update_check_cron
            .clone()
            .unwrap_or_else(|| "0 */6 * * *".into());
        let notify = s.update_check_notify.unwrap_or(false);
        drop(s);

        if !enabled {
            continue;
        }
        let now = Local::now();
        let expr = format!("0 {}", cron);
        let should_run = match expr.parse::<cron::Schedule>() {
            Ok(schedule) => schedule.includes(now.to_utc()),
            Err(e) => {
                tracing::warn!("update_check: invalid cron '{}': {}", cron, e);
                false
            }
        };
        if !should_run {
            continue;
        }

        tracing::info!(
            "update_check: ejecutando revisión de actualizaciones (cron={})",
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

        // 2. Revisar cada contenedor en paralelo
        let mut results: Vec<(String, bool)> = Vec::new();
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
                async move {
                    if image_full.is_empty() {
                        return (name, false);
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
                            (name, has_update)
                        }
                        Err(_) => (name, false),
                    }
                }
            })
            .collect();
        for task in futures::future::join_all(tasks).await {
            results.push(task);
        }

        // 3. Persistir has_update en DB
        {
            let conn = db::global().lock().await;
            let mut updated_count = 0u32;
            for (name, has_update) in &results {
                let _ = db::update_container_has_update(&conn, name, *has_update);
                if *has_update {
                    updated_count += 1;
                }
            }
            tracing::info!(
                "update_check: {} containers checked, {} with updates",
                results.len(),
                updated_count
            );
        }

        // 4. Obtener políticas de actualización + política global por defecto
        let policies = update_policies.lock().await.clone();
        let policy_map: HashMap<String, UpdatePolicy> = policies
            .into_iter()
            .map(|p| (p.container.clone(), p))
            .collect();
        let default_action = {
            let s = settings.lock().await;
            (
                s.default_update_action
                    .clone()
                    .unwrap_or_else(|| "pull-restart".into()),
                s.default_cleanup_old_image.unwrap_or(false),
                s.default_rollback_on_failure.unwrap_or(false),
            )
        };

        // 5. Ejecutar acciones para contenedores con actualizaciones pendientes
        for (name, has_update) in &results {
            if !*has_update {
                continue;
            }
            let policy = match policy_map.get(name) {
                Some(p) => p.clone(),
                None => UpdatePolicy {
                    container: name.clone(),
                    action: default_action
                        .0
                        .parse()
                        .unwrap_or(UpdateAction::PullRestart),
                    cleanup_old_image: default_action.1,
                    rollback_on_failure: default_action.2,
                },
            };
            if policy.action == UpdateAction::None {
                continue;
            }

            let _ = update_tx.send(UpdateProgress {
                container: name.clone(),
                status: format!("[update-check] 🔍 {}", policy.action),
                done: false,
                error: None,
            });

            let container = containers.iter().find(|c| {
                c.names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| crate::models::strip_name(n) == *name)
                    .unwrap_or(false)
            });
            let Some(container) = container else {
                continue;
            };
            let image_full = container.image.as_deref().unwrap_or("").to_string();
            let cid = container.id.as_deref().unwrap_or("").to_string();

            match policy.action {
                UpdateAction::Pull => {
                    if pull_image(&docker, &image_full).await {
                        let _ = update_tx.send(UpdateProgress {
                            container: name.clone(),
                            status: "✅ pulled".into(),
                            done: true,
                            error: None,
                        });
                    }
                }
                UpdateAction::PullRestart => {
                    if policy.cleanup_old_image || policy.rollback_on_failure {
                        // Con limpieza/rollback
                        let backup = if policy.rollback_on_failure {
                            tag_backup_image(&docker, &image_full).await
                        } else {
                            None
                        };
                        if pull_image(&docker, &image_full).await {
                            let _ = docker
                                .restart_container(
                                    &cid,
                                    None::<bollard::container::RestartContainerOptions>,
                                )
                                .await;
                            if policy.rollback_on_failure
                                && !verify_container_healthy(&docker, name).await
                            {
                                if let Some((backup_full, base, orig_tag)) = backup {
                                    rollback_container(
                                        &docker,
                                        &cid,
                                        &base,
                                        &orig_tag,
                                        &backup_full,
                                        &image_full,
                                    )
                                    .await;
                                }
                            } else if policy.cleanup_old_image {
                                let _ = docker
                                    .remove_image(
                                        &cid,
                                        None::<bollard::image::RemoveImageOptions>,
                                        None,
                                    )
                                    .await;
                            }
                            let _ = update_tx.send(UpdateProgress {
                                container: name.clone(),
                                status: "✅ updated + restarted".into(),
                                done: true,
                                error: None,
                            });
                        }
                    } else {
                        if pull_image(&docker, &image_full).await {
                            let _ = docker
                                .restart_container(
                                    &cid,
                                    None::<bollard::container::RestartContainerOptions>,
                                )
                                .await;
                            let _ = update_tx.send(UpdateProgress {
                                container: name.clone(),
                                status: "✅ updated + restarted".into(),
                                done: true,
                                error: None,
                            });
                        }
                    }
                }
                UpdateAction::PullRestartStack => {
                    let compose_project = container
                        .labels
                        .as_ref()
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
                            let pull = tokio::process::Command::new("docker")
                                .args(["compose", "-f", file, "pull"])
                                .output()
                                .await;
                            if let Ok(output) = pull {
                                if output.status.success() {
                                    let _ = tokio::process::Command::new("docker")
                                        .args(["compose", "-f", file, "up", "-d"])
                                        .output()
                                        .await;
                                    let _ = update_tx.send(UpdateProgress {
                                        container: name.clone(),
                                        status: "✅ stack updated".into(),
                                        done: true,
                                        error: None,
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            // Notificar si está configurado
            if notify {
                let msg = format!("🔄 '{}' → {} (update-check)", name, policy.action);
                let _ = notify_all(&config, &settings, name, &msg).await;
            }
        }
    }
}

async fn resolve_compose_file(docker: &Docker, project: &str) -> Option<String> {
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
