use bollard::{container::RestartContainerOptions, Docker};
use chrono::Local;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::containers::{find_container_by_name, pull_image};
use crate::models::*;
use crate::notifications::notify_all;
use crate::persistence::json_writer;
use crate::workers::state::docker_list_running;

/// Auto-update worker: periodic pull + restart
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
        for (name, image, cid, image_id) in docker_list_running(&docker).await {
            let (repo, local_tag) = if let Some(pos) = image.rfind('@') {
                (image[..pos].to_string(), "digest".to_string())
            } else if let Some(pos) = image.rfind(':') {
                (image[..pos].to_string(), image[pos + 1..].to_string())
            } else {
                (image.clone(), "latest".to_string())
            };

            let should_update = match crate::updates::check_remote_digest(&repo, &local_tag).await {
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
                Err(_) => true,
            };

            if !should_update {
                continue;
            }

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
        let _ = docker
            .prune_images(None::<bollard::image::PruneImagesOptions<&str>>)
            .await;
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
