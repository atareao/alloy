use axum::{
    extract::{Path, State},
    response::Json,
};
use bollard::{
    container::{ListContainersOptions, RestartContainerOptions},
    Docker,
};
use chrono::Local;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::containers::{fetch_containers, find_container_by_name, pull_image};
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::check_remote_digest;

pub async fn update_container_h(
    State(docker): State<Docker>,
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    Path(name): Path<String>,
) -> Result<Json<UpdateProgress>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image = container.image.as_deref().unwrap_or("");
    let cid = container.id.as_deref().unwrap_or("");
    let _ = update_tx.send(UpdateProgress {
        container: name.clone(),
        status: format!("Pulling {}...", image),
        done: false,
        error: None,
    });
    let start_time = std::time::Instant::now();
    if !pull_image(&docker, image).await {
        let _ = update_tx.send(UpdateProgress {
            container: name.clone(),
            status: "Error".into(),
            done: true,
            error: Some("pull failed".into()),
        });
        let entry = UpdateHistoryEntry {
            container: name.clone(),
            image: image.to_string(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            status: "error".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
        };
        let mut hist = update_history.lock().await;
        hist.push(entry);
        crate::persistence::json_writer()
            .save(FILE_UPDATES_HISTORY, &*hist)
            .await;
        return Err(AppError::Internal("pull failed".into()));
    }
    let _ = update_tx.send(UpdateProgress {
        container: name.clone(),
        status: "Restarting...".into(),
        done: false,
        error: None,
    });
    match docker
        .restart_container(cid, None::<RestartContainerOptions>)
        .await
    {
        Ok(_) => {
            let _ = update_tx.send(UpdateProgress {
                container: name.clone(),
                status: "✅ Restarted".into(),
                done: true,
                error: None,
            });
            let ts = Local::now().format("%H:%M:%S").to_string();
            let _ = notif_tx.send(NotifEvent {
                container: name.clone(),
                status: "updated ✅".into(),
                timestamp: ts,
            });
            notify_all(&config, &settings, &name, "✅ actualizado y reiniciado").await;
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.to_string(),
                old_digest: String::new(),
                new_digest: String::new(),
                timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                status: "success".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            crate::persistence::json_writer()
                .save(FILE_UPDATES_HISTORY, &*hist)
                .await;
            Ok(Json(UpdateProgress {
                container: name,
                status: "ok".into(),
                done: true,
                error: None,
            }))
        }
        Err(e) => {
            let _ = update_tx.send(UpdateProgress {
                container: name.clone(),
                status: "Error".into(),
                done: true,
                error: Some(e.to_string()),
            });
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.to_string(),
                old_digest: String::new(),
                new_digest: String::new(),
                timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            crate::persistence::json_writer()
                .save(FILE_UPDATES_HISTORY, &*hist)
                .await;
            Err(AppError::Docker(format!("restart: {}", e)))
        }
    }
}

pub async fn update_all_h(
    State(docker): State<Docker>,
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<Vec<UpdateProgress>> {
    let mut results = vec![];
    for (name, image, cid, _) in crate::workers::docker_list_running(&docker).await {
        let start_time = std::time::Instant::now();
        if !pull_image(&docker, &image).await {
            results.push(UpdateProgress {
                container: name.clone(),
                status: "error".into(),
                done: true,
                error: Some("pull failed".into()),
            });
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.clone(),
                old_digest: String::new(),
                new_digest: String::new(),
                timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            crate::persistence::json_writer()
                .save(FILE_UPDATES_HISTORY, &*hist)
                .await;
            continue;
        }
        match docker
            .restart_container(&cid, None::<RestartContainerOptions>)
            .await
        {
            Ok(_) => {
                let ts = Local::now().format("%H:%M:%S").to_string();
                let _ = notif_tx.send(NotifEvent {
                    container: name.clone(),
                    status: "updated ✅".into(),
                    timestamp: ts,
                });
                notify_all(&config, &settings, &name, "✅ actualizado").await;
                results.push(UpdateProgress {
                    container: name.clone(),
                    status: "ok".into(),
                    done: true,
                    error: None,
                });
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest: String::new(),
                    new_digest: String::new(),
                    timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                    status: "success".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                crate::persistence::json_writer()
                    .save(FILE_UPDATES_HISTORY, &*hist)
                    .await;
            }
            Err(e) => {
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
    Path(name): Path<String>,
) -> Result<Json<VersionCompare>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image_full = container.image.as_deref().unwrap_or("");
    if image_full.is_empty() {
        return Ok(Json(VersionCompare {
            local_tag: "unknown".into(),
            remote_tag: None,
            has_update: None,
            local_digest: None,
            remote_digest: None,
            changelog_url: None,
            error: Some("no image".into()),
        }));
    }
    let (repo, local_tag) = crate::updates::digest::parse_image_ref(image_full);
    let (remote_digest, remote_tag, error) = match check_remote_digest(&repo, &local_tag).await {
        Ok((digest, tag)) => (Some(digest), Some(tag), None),
        Err(e) => (None, None, Some(e)),
    };
    let has_update = match (&container.image_id, &remote_digest) {
        (Some(local_digest), Some(remote_digest)) => {
            let local_short = crate::updates::digest::short_digest(local_digest);
            let remote_short = crate::updates::digest::short_digest(remote_digest);
            Some(local_short != remote_short)
        }
        _ => None,
    };
    let local_digest = container
        .image_id
        .as_ref()
        .map(|d| crate::updates::digest::short_digest(d));
    let changelog_url = if repo.contains('/') {
        Some(format!("https://hub.docker.com/r/{}/tags", repo))
    } else {
        Some(format!("https://hub.docker.com/_/{}/tags", repo))
    };
    Ok(Json(VersionCompare {
        local_tag,
        remote_tag,
        has_update,
        local_digest,
        remote_digest: remote_digest.map(|d| crate::updates::digest::short_digest(&d)),
        changelog_url,
        error,
    }))
}

pub async fn check_all_h(
    State(docker): State<Docker>,
    State(config): State<Config>,
) -> Json<Vec<ContainerInfo>> {
    let mut containers = fetch_containers(&docker, &config.allowed_containers).await;
    let tasks: Vec<_> = containers
        .iter()
        .map(|c| {
            let docker = docker.clone();
            let name = c.name.clone();
            async move {
                let containers = docker
                    .list_containers(Some(ListContainersOptions::<String> {
                        all: true,
                        ..Default::default()
                    }))
                    .await
                    .unwrap_or_default();
                if let Some(container) = containers.iter().find(|ct| {
                    ct.names
                        .as_ref()
                        .and_then(|n| n.first())
                        .map(|n| strip_name(n) == name.as_str())
                        .unwrap_or(false)
                }) {
                    let image_full = container.image.as_deref().unwrap_or("");
                    if image_full.is_empty() {
                        return (name, false);
                    }
                    let (repo, local_tag) = crate::updates::digest::parse_image_ref(image_full);
                    match check_remote_digest(&repo, &local_tag).await {
                        Ok((remote_digest, _)) => {
                            let has_update = container
                                .image_id
                                .as_ref()
                                .map(|local_digest| {
                                    let local_short =
                                        crate::updates::digest::short_digest(local_digest);
                                    let remote_short =
                                        crate::updates::digest::short_digest(&remote_digest);
                                    local_short != remote_short
                                })
                                .unwrap_or(false);
                            (name, has_update)
                        }
                        Err(_) => (name, false),
                    }
                } else {
                    (name, false)
                }
            }
        })
        .collect();
    let results = futures::future::join_all(tasks).await;
    let update_map: std::collections::HashMap<String, bool> = results.into_iter().collect();
    for c in &mut containers {
        c.has_update = *update_map.get(&c.name).unwrap_or(&false);
    }
    Json(containers)
}
