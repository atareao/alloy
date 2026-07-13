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

async fn update_all_h(
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

/// Fetch the config digest (image ID) of a remote image from Docker Hub.
///
/// Returns `(config_digest, tag)` where `config_digest` matches what Docker
/// stores locally as `ImageID`, so a byte-for-byte comparison is correct.
///
/// For multi-arch (manifest list) images this performs a second request to
/// resolve the platform-specific manifest and extract its `config.digest`.
pub async fn check_remote_digest(repo: &str, tag: &str) -> Result<(String, String), String> {
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

    // Step 1: fetch the manifest (or manifest list) for the given tag
    let manifest_url = format!("https://registry-1.docker.io/v2/{}/manifests/{}", repo, tag);
    let manifest_resp = client
        .get(&manifest_url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.list.v2+json",
        )
        .header("Accept", "application/vnd.oci.image.manifest.v1+json")
        .header("Accept", "application/vnd.oci.image.index.v1+json")
        .send()
        .await
        .map_err(|e| format!("manifest request failed: {}", e))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("manifest status: {}", manifest_resp.status()));
    }

    let content_type = manifest_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Step 2: extract the config digest (image ID), resolving manifest lists
    let config_digest =
        if content_type.contains("manifest.list") || content_type.contains("image.index") {
            // Multi-arch image: grab the first linux/amd64 manifest digest
            let body: serde_json::Value = manifest_resp
                .json()
                .await
                .map_err(|e| format!("manifest list parse failed: {}", e))?;
            let manifests = body["manifests"]
                .as_array()
                .ok_or_else(|| "no manifests in list".to_string())?;
            let amd64_digest = manifests
                .iter()
                .find(|m| {
                    let plat = &m["platform"];
                    plat["architecture"].as_str() == Some("amd64")
                        && plat["os"].as_str() == Some("linux")
                })
                .or_else(|| manifests.first())
                .and_then(|m| m["digest"].as_str())
                .ok_or_else(|| "no suitable platform manifest".to_string())?;

            // Fetch the platform-specific manifest
            let plat_url = format!(
                "https://registry-1.docker.io/v2/{}/manifests/{}",
                repo, amd64_digest
            );
            let plat_resp = client
                .get(&plat_url)
                .header("Authorization", format!("Bearer {}", token))
                .header(
                    "Accept",
                    "application/vnd.docker.distribution.manifest.v2+json",
                )
                .header("Accept", "application/vnd.oci.image.manifest.v1+json")
                .send()
                .await
                .map_err(|e| format!("platform manifest request failed: {}", e))?;
            if !plat_resp.status().is_success() {
                return Err(format!("platform manifest status: {}", plat_resp.status()));
            }
            let plat_body: serde_json::Value = plat_resp
                .json()
                .await
                .map_err(|e| format!("platform manifest parse failed: {}", e))?;
            plat_body["config"]["digest"]
                .as_str()
                .ok_or_else(|| "no config digest in platform manifest".to_string())?
                .to_string()
        } else {
            // Single-arch manifest: extract config.digest directly
            let body: serde_json::Value = manifest_resp
                .json()
                .await
                .map_err(|e| format!("manifest parse failed: {}", e))?;
            body["config"]["digest"]
                .as_str()
                .ok_or_else(|| "no config digest".to_string())?
                .to_string()
        };

    Ok((config_digest, tag.to_string()))
}

// ── History handlers (moved from containers.rs) ────────────
use axum::routing::get;

async fn get_history_h(
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<Vec<UpdateHistoryEntry>> {
    let hist = update_history.lock().await;
    let mut sorted = hist.clone();
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    sorted.truncate(100);
    Json(sorted)
}

async fn delete_history_h(
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<serde_json::Value> {
    let mut hist = update_history.lock().await;
    hist.clear();
    crate::persistence::json_writer()
        .save(FILE_UPDATES_HISTORY, &*hist)
        .await;
    Json(serde_json::json!({"status": "cleared"}))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/update/{name}", post(update_container_h))
        .route("/api/update-all", post(update_all_h))
        .route("/api/check-update/{name}", post(check_update_h))
        .route("/api/check-all", post(check_all_h))
        .route("/api/history", get(get_history_h).delete(delete_history_h))
}
