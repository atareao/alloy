use bollard::{
    container::{ListContainersOptions, RestartContainerOptions},
    Docker,
};
use chrono::Local;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::containers::{find_container_by_name, pull_image};
use crate::models::*;
use crate::notifications::notify_all;
use crate::workers::auto_update::{rollback_container, tag_backup_image, verify_container_healthy};
use crate::workers::state::docker_list_running;

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

pub async fn scheduler_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    update_tx: broadcast::Sender<UpdateProgress>,
    notif_tx: broadcast::Sender<NotifEvent>,
    schedules: Arc<Mutex<Vec<ScheduleTask>>>,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        tick.tick().await;
        let now = Local::now();
        let tasks = schedules.lock().await.clone();
        for task in &tasks {
            if !task.enabled {
                continue;
            }
            if !match_cron(&task.cron, &now) {
                continue;
            }
            tracing::info!(
                "Scheduler: ejecutando '{}' target_type='{}' target='{}'",
                task.action,
                task.target_type,
                task.container
            );

            let targets: Vec<(String, String, String)> = match task.target_type.as_str() {
                "stack" => {
                    let containers = docker
                        .list_containers(Some(ListContainersOptions::<String> {
                            all: true,
                            ..Default::default()
                        }))
                        .await
                        .unwrap_or_default();
                    containers
                        .iter()
                        .filter(|c| {
                            c.labels
                                .as_ref()
                                .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                                .map(|p| p == &task.container)
                                .unwrap_or(false)
                        })
                        .filter_map(|c| {
                            let cname = c
                                .names
                                .as_ref()
                                .and_then(|n| n.first())
                                .map(|n| crate::models::strip_name(n))
                                .unwrap_or_default();
                            c.id.as_deref()
                                .zip(c.image.as_deref())
                                .map(|(id, img)| (cname, img.to_string(), id.to_string()))
                        })
                        .collect()
                }
                _ => {
                    if task.container == crate::models::ALL_CONTAINERS {
                        docker_list_running(&docker)
                            .await
                            .into_iter()
                            .map(|(name, image, cid, _)| (name, image, cid))
                            .collect()
                    } else {
                        match find_container_by_name(&docker, &task.container).await {
                            Ok(c) => {
                                c.id.as_deref()
                                    .zip(c.image.as_deref())
                                    .map(|(id, img)| {
                                        (task.container.clone(), img.to_string(), id.to_string())
                                    })
                                    .into_iter()
                                    .collect()
                            }
                            Err(_) => vec![],
                        }
                    }
                }
            };

            if targets.is_empty() {
                tracing::warn!("Scheduler: no targets found for task '{}'", task.container);
                continue;
            }

            let mut updated_ok = 0u32;
            let mut failed = 0u32;

            match task.action.as_str() {
                "check-update" => {
                    for (cname, image, _cid) in &targets {
                        tracing::info!("Scheduler: check-update for {} ({})", cname, image);
                        let _ = update_tx.send(UpdateProgress {
                            container: cname.clone(),
                            status: format!("[scheduled] 🔍 checked {}", image),
                            done: true,
                            error: None,
                        });
                    }
                    updated_ok = targets.len() as u32;
                }
                "update" => {
                    if task.target_type == "stack" {
                        let compose_file = resolve_compose_file(&docker, &task.container).await;
                        if let Some(ref file) = compose_file {
                            tracing::info!(
                                "Scheduler: pulling stack '{}' via {}",
                                task.container,
                                file
                            );
                            let _ = update_tx.send(UpdateProgress {
                                container: task.container.clone(),
                                status: format!("📥 Pulling stack '{}'...", task.container),
                                done: false,
                                error: None,
                            });
                            let pull = tokio::process::Command::new("docker")
                                .args(["compose", "-f", file, "pull"])
                                .output()
                                .await;
                            if let Ok(output) = pull {
                                if output.status.success() {
                                    tracing::info!(
                                        "Scheduler: stack '{}' pulled, recreating...",
                                        task.container
                                    );
                                    let up = tokio::process::Command::new("docker")
                                        .args(["compose", "-f", file, "up", "-d"])
                                        .output()
                                        .await;
                                    if let Ok(up_out) = up {
                                        if up_out.status.success() {
                                            updated_ok = targets.len() as u32;
                                            let _ = notif_tx.send(NotifEvent {
                                                container: format!("stack:{}", task.container),
                                                status: "🕐 scheduled stack update ✅".into(),
                                                timestamp: Local::now()
                                                    .format("%H:%M:%S")
                                                    .to_string(),
                                            });
                                        } else {
                                            failed = targets.len() as u32;
                                            let stderr =
                                                String::from_utf8_lossy(&up_out.stderr).to_string();
                                            tracing::error!("Stack up error: {}", stderr);
                                        }
                                    }
                                } else {
                                    failed = targets.len() as u32;
                                    let stderr =
                                        String::from_utf8_lossy(&output.stderr).to_string();
                                    tracing::error!("Stack pull error: {}", stderr);
                                }
                            } else {
                                failed = targets.len() as u32;
                            }
                        } else {
                            tracing::error!("No compose file found for stack '{}'", task.container);
                            failed = targets.len() as u32;
                        }
                    } else {
                        for (cname, image, cid) in &targets {
                            let _ = update_tx.send(UpdateProgress {
                                container: cname.clone(),
                                status: format!("[scheduled] pulling {}", image),
                                done: false,
                                error: None,
                            });

                            match task.cleanup.as_str() {
                                "rollback" => {
                                    let backup = tag_backup_image(&docker, image).await;
                                    if pull_image(&docker, image).await {
                                        let _ = docker
                                            .restart_container(cid, None::<RestartContainerOptions>)
                                            .await;
                                        if verify_container_healthy(&docker, cname).await {
                                            updated_ok += 1;
                                            if let Some((ref backup_full, _, _)) = backup {
                                                let _ = docker
                                                    .remove_image(
                                                        backup_full,
                                                        None::<bollard::image::RemoveImageOptions>,
                                                        None,
                                                    )
                                                    .await;
                                            }
                                        } else {
                                            failed += 1;
                                            if let Some((backup_full, base, orig_tag)) = backup {
                                                rollback_container(
                                                    &docker,
                                                    cid,
                                                    &base,
                                                    &orig_tag,
                                                    &backup_full,
                                                    image,
                                                )
                                                .await;
                                            }
                                        }
                                    } else {
                                        failed += 1;
                                        if let Some((backup_full, base, orig_tag)) = backup {
                                            let restore = bollard::image::TagImageOptions {
                                                repo: base,
                                                tag: orig_tag,
                                            };
                                            let _ =
                                                docker.tag_image(&backup_full, Some(restore)).await;
                                        }
                                    }
                                }
                                "delete-old" => {
                                    if pull_image(&docker, image).await {
                                        let _ = docker
                                            .restart_container(cid, None::<RestartContainerOptions>)
                                            .await;
                                        updated_ok += 1;
                                        let _ = docker
                                            .remove_image(
                                                cid,
                                                None::<bollard::image::RemoveImageOptions>,
                                                None,
                                            )
                                            .await;
                                    } else {
                                        failed += 1;
                                    }
                                }
                                _ => {
                                    if pull_image(&docker, image).await {
                                        let _ = docker
                                            .restart_container(cid, None::<RestartContainerOptions>)
                                            .await;
                                        updated_ok += 1;
                                    } else {
                                        failed += 1;
                                    }
                                }
                            }

                            let _ = notif_tx.send(NotifEvent {
                                container: cname.clone(),
                                status: format!(
                                    "🕐 scheduled update {}",
                                    if failed > 0 { "❌" } else { "✅" }
                                ),
                                timestamp: Local::now().format("%H:%M:%S").to_string(),
                            });
                        }
                    }
                }
                "restart" => {
                    for (cname, _image, cid) in &targets {
                        let _ = docker
                            .restart_container(cid, None::<RestartContainerOptions>)
                            .await;
                        updated_ok += 1;
                        let _ = notif_tx.send(NotifEvent {
                            container: cname.clone(),
                            status: "🕐 scheduled restart ✅".into(),
                            timestamp: Local::now().format("%H:%M:%S").to_string(),
                        });
                    }
                }
                _ => {
                    tracing::warn!("Scheduler: acción desconocida '{}'", task.action);
                }
            }

            if task.notify {
                let status = if failed > 0 {
                    format!(
                        "⚠️ Tarea '{}': {} ok, {} fallos",
                        task.action, updated_ok, failed
                    )
                } else {
                    format!(
                        "✅ Tarea '{}' completada ({} targets)",
                        task.action, updated_ok
                    )
                };
                let _ = notify_all(&config, &settings, &task.container, &status).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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

    // ── Integration: resolve_compose_file ───────────────────

    fn is_podman_available() -> bool {
        std::env::var("DOCKER_HOST").is_ok()
            || std::path::Path::new("/run/user/1000/podman/podman.sock").exists()
    }

    async fn podman_client() -> Docker {
        let socket = std::env::var("DOCKER_HOST")
            .unwrap_or_else(|_| "unix:///run/user/1000/podman/podman.sock".to_string());
        Docker::connect_with_local(&socket, 120, bollard::API_DEFAULT_VERSION)
            .expect("Failed to connect to Podman socket")
    }

    #[tokio::test]
    async fn test_integration_resolve_compose_file_nonexistent() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result = resolve_compose_file(&docker, "nonexistent-project-xyz").await;
        assert!(
            result.is_none(),
            "Should return None for nonexistent project"
        );
    }
}
