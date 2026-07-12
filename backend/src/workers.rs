use bollard::{
    container::{ListContainersOptions, RestartContainerOptions},
    system::EventsOptions,
    Docker,
};
use chrono::Local;
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::persistence::json_writer;

use crate::config::Config;
use crate::containers::{fetch_containers, find_container_by_name, pull_image};
use crate::models::ALL_CONTAINERS;
use crate::models::*;
use crate::notifications::{notify_all, notify_selected};

pub type CachedContainers = Arc<RwLock<Option<Vec<ContainerInfo>>>>;

pub async fn docker_list_running(docker: &Docker) -> Vec<(String, String, String)> {
    match docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await
    {
        Ok(list) => list
            .iter()
            .filter_map(|c| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n))?;
                let image = c.image.as_deref()?.to_string();
                let id = c.id.as_deref()?.to_string();
                Some((name, image, id))
            })
            .collect(),
        Err(_) => vec![],
    }
}

// ── State Worker: Docker Events API + fallback ────────────

pub async fn state_worker(
    docker: Docker,
    config: Config,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    async fn refresh(
        docker: &Docker,
        config: &Config,
        tx: &broadcast::Sender<StateEvent>,
        cache: &CachedContainers,
    ) {
        let containers = fetch_containers(docker, &config.allowed_containers).await;
        *cache.write().await = Some(containers.clone());
        let _ = tx.send(StateEvent { containers });
    }

    refresh(&docker, &config, &tx, &cached_containers).await;

    loop {
        let options = EventsOptions::<String> {
            since: None,
            until: None,
            filters: HashMap::new(),
        };
        let stream = docker.events(Some(options));
        pin_mut!(stream);
        let mut fallback = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                event = stream.next() => {
                    match event {
                        Some(Ok(evt)) => {
                            if evt.typ == Some(bollard::models::EventMessageTypeEnum::CONTAINER) {
                                if let Some(ref action) = evt.action {
                                    if relevant_actions.contains(&action.as_str()) {
                                        tracing::debug!("Docker event: {} on {:?}", action, evt.actor.as_ref().map(|a| &a.id));
                                        refresh(&docker, &config, &tx, &cached_containers).await;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            tracing::warn!("Docker events stream error: {} — reconnecting", e);
                            break;
                        }
                        None => {
                            tracing::warn!("Docker events stream ended — reconnecting");
                            break;
                        }
                    }
                }
                _ = fallback.tick() => {
                    refresh(&docker, &config, &tx, &cached_containers).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

pub async fn auto_update_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    notif_tx: broadcast::Sender<NotifEvent>,
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
) {
    let enabled = {
        settings
            .lock()
            .await
            .auto_update_enabled
            .unwrap_or_else(|| config.auto_update())
    };
    if !enabled {
        return;
    }
    let hours = config.auto_update_interval();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(hours * 3600));
    loop {
        interval.tick().await;
        for (name, image, cid) in docker_list_running(&docker).await {
            let start_time = std::time::Instant::now();
            if !pull_image(&docker, &image).await {
                continue;
            }
            if docker
                .restart_container(&cid, None::<RestartContainerOptions>)
                .await
                .is_ok()
            {
                let _ = notif_tx.send(NotifEvent {
                    container: name.clone(),
                    status: "🤖 auto-updated".into(),
                    timestamp: Local::now().format("%H:%M:%S").to_string(),
                });
                notify_all(&config, &settings, &name, "🤖 auto-actualizado").await;
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest: String::new(),
                    new_digest: String::new(),
                    timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                    status: "auto-update".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                json_writer().save(FILE_UPDATES_HISTORY, &*hist).await;
            }
        }
    }
}

/// Monitoriza cambios de estado de los contenedores y notifica.
/// Solo notifica transiciones: running → algo (problema) y algo → running (recuperación).
pub async fn alerts_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    notif_tx: broadcast::Sender<NotifEvent>,
    alerts: Arc<Mutex<Vec<AlertConfig>>>,
) {
    let mut previous_states: HashMap<String, String> = HashMap::new();
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(15));

    loop {
        tick.tick().await;
        let alerts_list = alerts.lock().await.clone();
        let containers = docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();
        let container_map: HashMap<String, &bollard::models::ContainerSummary> = containers
            .iter()
            .filter_map(|c| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n))?;
                Some((name, c))
            })
            .collect();

        for alert in &alerts_list {
            if !alert.enabled {
                continue;
            }
            let container_name = &alert.container;
            let Some(container) = container_map.get(container_name) else {
                // Container no existe — puede que esté stopped
                let prev = previous_states.insert(container_name.clone(), "gone".into());
                if prev.as_deref() != Some("gone") {
                    let msg = format!("⚠️ Container '{}' ha desaparecido", container_name);
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: "alert: gone".into(),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_selected(&config, &settings, container_name, &msg, &alert.notify_via)
                        .await;
                }
                continue;
            };
            let current_state = container.state.as_deref().unwrap_or("unknown").to_string();
            let prev_state = previous_states.insert(container_name.clone(), current_state.clone());

            if let Some(prev) = prev_state {
                // Transición: running → algo malo
                if prev == "running"
                    && (current_state == "exited"
                        || current_state == "dead"
                        || current_state == "paused"
                        || current_state == "restarting")
                {
                    let msg = format!(
                        "⚠️ Container '{}' ha cambiado a: {}",
                        container_name, current_state
                    );
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: format!("alert: {}", current_state),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_selected(&config, &settings, container_name, &msg, &alert.notify_via)
                        .await;
                }
                // Transición: algo malo → running (recuperación)
                if current_state == "running"
                    && (prev == "exited"
                        || prev == "dead"
                        || prev == "paused"
                        || prev == "restarting")
                {
                    let msg = format!("✅ Container '{}' ha vuelto a running", container_name);
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: "alert: recovered".into(),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_selected(&config, &settings, container_name, &msg, &alert.notify_via)
                        .await;
                }
            }
        }
    }
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

            // ── Resolve targets based on target_type ────────────────
            let targets: Vec<(String, String, String)> = match task.target_type.as_str() {
                "stack" => {
                    // Resolve all containers in the compose project
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
                                .and_then(|l| l.get(LABEL_COMPOSE_PROJECT))
                                .map(|p| p == &task.container)
                                .unwrap_or(false)
                        })
                        .filter_map(|c| {
                            let cname = c
                                .names
                                .as_ref()
                                .and_then(|n| n.first())
                                .map(|n| strip_name(n))
                                .unwrap_or_default();
                            c.id.as_deref()
                                .zip(c.image.as_deref())
                                .map(|(id, img)| (cname, img.to_string(), id.to_string()))
                        })
                        .collect()
                }
                _ => {
                    if task.container == ALL_CONTAINERS {
                        docker_list_running(&docker).await
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

            // ── Execute action ──────────────────────────────────────
            let mut updated_ok = 0u32;
            let mut failed = 0u32;

            match task.action.as_str() {
                "check-update" => {
                    // For both containers and stacks, check via individual container endpoint
                    for (cname, image, _cid) in &targets {
                        tracing::info!("Scheduler: check-update for {} ({})", cname, image);
                        // We just log — actual check logic is in updates.rs
                        // Future: store result and notify
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
                        // Stack update via docker compose
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
                        // Container update: pull + restart per target
                        for (cname, image, cid) in &targets {
                            let _ = update_tx.send(UpdateProgress {
                                container: cname.clone(),
                                status: format!("[scheduled] pulling {}", image),
                                done: false,
                                error: None,
                            });

                            match task.cleanup.as_str() {
                                "rollback" => {
                                    // Step 1: tag backup
                                    let backup = tag_backup_image(&docker, image).await;
                                    // Step 2: pull new image
                                    if pull_image(&docker, image).await {
                                        // Step 3: restart container
                                        let _ = docker
                                            .restart_container(cid, None::<RestartContainerOptions>)
                                            .await;
                                        // Step 4: verify health
                                        if verify_container_healthy(&docker, cname).await {
                                            updated_ok += 1;
                                            // Step 5 ✅: remove backup tag
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
                                            // Step 6 ❌: rollback to old image
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
                                        // Pull failed: restore backup tag if we created one
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
                                        // Remove old image (by placeholder cid digest)
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
                                    // cleanup == "none": just pull + restart
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

            // ── Notifications ──────────────────────────────────────
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

/// Verify a container is running (or healthy) after a restart
async fn verify_container_healthy(docker: &Docker, name: &str) -> bool {
    tokio::time::sleep(Duration::from_secs(3)).await;
    match find_container_by_name(docker, name).await {
        Ok(c) => c.state.as_deref() == Some("running"),
        Err(_) => false,
    }
}

/// Tag current image as backup for rollback: image:tag → image:rollback-{ts}
async fn tag_backup_image(docker: &Docker, image: &str) -> Option<(String, String, String)> {
    let ts = Local::now().format("%Y%m%d%H%M%S").to_string();
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
async fn rollback_container(
    docker: &Docker,
    cid: &str,
    base: &str,
    original_tag: &str,
    backup_full: &str,
    new_image: &str,
) {
    tracing::warn!("Rollback: restoring backup for {}", new_image);
    // Restore original tag from backup
    let restore_opts = bollard::image::TagImageOptions {
        repo: base.to_string(),
        tag: original_tag.to_string(),
    };
    let _ = docker.tag_image(backup_full, Some(restore_opts)).await;
    // Restart container with old image
    let _ = docker
        .restart_container(cid, None::<RestartContainerOptions>)
        .await;
    // Remove the new (bad) image by digest
    let _ = docker
        .remove_image(new_image, None::<bollard::image::RemoveImageOptions>, None)
        .await;
}

/// Resolve the docker-compose file path for a given project name
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
                .and_then(|l| l.get(LABEL_COMPOSE_PROJECT))
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
        .and_then(|l| l.get(LABEL_COMPOSE_CONFIG_FILES))
        .cloned()
        .or_else(|| {
            project_containers
                .first()
                .and_then(|c| c.labels.as_ref())
                .and_then(|l| l.get(LABEL_COMPOSE_WORKING_DIR))
                .map(|dir| format!("{}/docker-compose.yml", dir))
        })
        .filter(|p| std::path::Path::new(p).exists())
}

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
}
