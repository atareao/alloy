use axum::{
    extract::{Path, State},
    response::Json,
    routing::post,
    Router,
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
use crate::state::{http_client, AppState};

async fn update_container_h(
    State(docker): State<Docker>,
    State(config): State<Config>,
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
        crate::workers::json_writer()
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
            notify_all(&config, &name, "✅ actualizado y reiniciado").await;
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
            crate::workers::json_writer()
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
            crate::workers::json_writer()
                .save(FILE_UPDATES_HISTORY, &*hist)
                .await;
            Err(AppError::Docker(format!("restart: {}", e)))
        }
    }
}

async fn update_all_h(
    State(docker): State<Docker>,
    State(config): State<Config>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<Vec<UpdateProgress>> {
    let mut results = vec![];
    for (name, image, cid) in crate::workers::docker_list_running(&docker).await {
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
            crate::workers::json_writer()
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
                notify_all(&config, &name, "✅ actualizado").await;
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
                crate::workers::json_writer()
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

async fn check_update_h(
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
    let (repo, local_tag) = if let Some(pos) = image_full.rfind('@') {
        (image_full[..pos].to_string(), "digest".to_string())
    } else if let Some(pos) = image_full.rfind(':') {
        (
            image_full[..pos].to_string(),
            image_full[pos + 1..].to_string(),
        )
    } else {
        (image_full.to_string(), "latest".to_string())
    };
    let (remote_digest, remote_tag, error) = match check_remote_digest(&repo, &local_tag).await {
        Ok((digest, tag)) => (Some(digest), Some(tag), None),
        Err(e) => (None, None, Some(e)),
    };
    let has_update = match (&container.image_id, &remote_digest) {
        (Some(local_digest), Some(remote_digest)) => {
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
            Some(local_short != remote_short)
        }
        _ => None,
    };
    let local_digest = container.image_id.as_ref().map(|d| {
        d.split(':')
            .next_back()
            .unwrap_or("")
            .chars()
            .take(12)
            .collect::<String>()
    });
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
        remote_digest: remote_digest.map(|d| {
            d.split(':')
                .next_back()
                .unwrap_or("")
                .chars()
                .take(12)
                .collect::<String>()
        }),
        changelog_url,
        error,
    }))
}

async fn check_all_h(
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
                    let (repo, local_tag) = if let Some(pos) = image_full.rfind(':') {
                        (
                            image_full[..pos].to_string(),
                            image_full[pos + 1..].to_string(),
                        )
                    } else {
                        (image_full.to_string(), "latest".to_string())
                    };
                    match check_remote_digest(&repo, &local_tag).await {
                        Ok((remote_digest, _)) => {
                            let has_update = container
                                .image_id
                                .as_ref()
                                .map(|local_digest| {
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

async fn check_remote_digest(repo: &str, tag: &str) -> Result<(String, String), String> {
    let client = http_client();
    let token_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
        repo
    );
    let token_resp = client
        .get(&token_url)
        .send()
        .await
        .map_err(|e| format!("token request failed: {}", e))?;
    let token_body: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|e| format!("token parse failed: {}", e))?;
    let token = token_body["token"]
        .as_str()
        .ok_or_else(|| "no token".to_string())?;
    let manifest_url = format!("https://registry-1.docker.io/v2/{}/manifests/{}", repo, tag);
    let manifest_resp = client
        .get(&manifest_url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header("Accept", "application/vnd.oci.image.manifest.v1+json")
        .send()
        .await
        .map_err(|e| format!("manifest request failed: {}", e))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("manifest status: {}", manifest_resp.status()));
    }
    let digest = manifest_resp
        .headers()
        .get("docker-content-digest")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| "no digest header".to_string())?;
    Ok((digest, tag.to_string()))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/update/{name}", post(update_container_h))
        .route("/api/update-all", post(update_all_h))
        .route("/api/check-update/{name}", post(check_update_h))
        .route("/api/check-all", post(check_all_h))
}
