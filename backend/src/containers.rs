use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, RemoveContainerOptions,
        RestartContainerOptions, StartContainerOptions, StopContainerOptions,
    },
    image::CreateImageOptions,
    Docker,
};
use futures::{pin_mut, StreamExt};

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::models::*;
use crate::state::AppState;
use crate::workers::{json_writer, CachedContainers};

pub async fn find_container_by_name(
    docker: &Docker,
    name: &str,
) -> Result<bollard::models::ContainerSummary, AppError> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(|e| AppError::Docker(e.to_string()))?;
    containers
        .into_iter()
        .find(|c| {
            c.names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| strip_name(n) == name)
                .unwrap_or(false)
        })
        .ok_or_else(|| AppError::NotFound(format!("Container '{}' not found", name)))
}

pub async fn fetch_containers(
    docker: &Docker,
    allowed: &Option<Vec<String>>,
) -> Vec<ContainerInfo> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            size: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();

    // Step 1: Pre-resolve bare digest images (async) before building ContainerInfo
    let mut resolved_images: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for c in &containers {
        let image = c.image.as_deref().unwrap_or("");
        if image.starts_with("sha256:") {
            if let Some(cid) = &c.id {
                if let Ok(inspect) = docker
                    .inspect_container(cid, None::<InspectContainerOptions>)
                    .await
                {
                    let real = inspect
                        .config
                        .as_ref()
                        .and_then(|cfg| cfg.image.as_deref())
                        .unwrap_or("");
                    resolved_images.insert(cid.clone(), real.to_string());
                }
            }
        }
    }

    // Step 2: Build ContainerInfo list (now all image names are resolved)
    containers
        .iter()
        .filter_map(|c| {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| strip_name(n))
                .unwrap_or_default();
            if let Some(allowed) = allowed {
                if !allowed.contains(&name) {
                    return None;
                }
            }

            // Determine the effective image reference string
            let raw_image = c.image.as_deref().unwrap_or("unknown");
            let effective_image = if raw_image.starts_with("sha256:") {
                // Resolved via inspect in step 1, or fall back to raw
                c.id.as_ref()
                    .and_then(|cid| resolved_images.get(cid))
                    .map(|s| s.as_str())
                    .unwrap_or(raw_image)
            } else {
                raw_image
            };

            // Parse image name + tag from effective_image
            let (image_name, tag) = if let Some(pos) = effective_image.find('@') {
                (effective_image[..pos].to_string(), String::new())
            } else if let Some((n, t)) = effective_image.rsplit_once(':') {
                (n.to_string(), t.to_string())
            } else {
                (effective_image.to_string(), "latest".into())
            };

            let ports: Vec<String> = c
                .ports
                .as_ref()
                .map(|ps| {
                    ps.iter()
                        .filter_map(|p| {
                            let pub_str =
                                p.public_port.map(|pp| pp.to_string()).unwrap_or_default();
                            if pub_str.is_empty() {
                                None
                            } else {
                                Some(format!(
                                    "{}:{}:{}",
                                    p.ip.as_deref().unwrap_or("0.0.0.0"),
                                    pub_str,
                                    p.private_port
                                ))
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            let traefik_url = c.labels.as_ref().and_then(|labels| {
                for (k, v) in labels {
                    if k.ends_with(".rule") && v.starts_with("Host(") {
                        let host = v
                            .trim_start_matches("Host(`")
                            .split('`')
                            .next()
                            .unwrap_or("");
                        let tls = labels
                            .iter()
                            .any(|(lk, lv)| lk.starts_with(&k[..k.len() - 5]) && lv == "true");
                        let proto = if tls { "https" } else { "http" };
                        return Some(format!("{}://{}", proto, host));
                    }
                }
                None
            });

            // Use the ORIGINAL raw_image for registry_url (the digest is fine for links)
            let registry_image = raw_image;
            let registry_url = if registry_image.contains('/') {
                if registry_image.starts_with("docker.io/") || !registry_image.contains('.') {
                    let parts: Vec<&str> = registry_image.splitn(2, '/').collect();
                    if parts.len() == 2 && parts[0].contains('.') {
                        format!("https://{}", parts[0])
                    } else {
                        let repo = registry_image.trim_start_matches("docker.io/");
                        if repo.contains('/') {
                            format!("https://hub.docker.com/r/{}", repo)
                        } else {
                            format!("https://hub.docker.com/_/{}/tags", repo)
                        }
                    }
                } else if registry_image.contains('.') {
                    format!("https://{}", registry_image.split('/').next().unwrap_or(""))
                } else {
                    format!("https://hub.docker.com/r/{}/tags", registry_image)
                }
            } else if registry_image.starts_with("sha256:") {
                // Bare digest — use the resolved name for registry URL
                if let Some(repo_name) = image_name.strip_prefix("docker.io/").or(Some(&image_name))
                {
                    if repo_name.contains('/') {
                        format!("https://hub.docker.com/r/{}", repo_name)
                    } else {
                        format!("https://hub.docker.com/_/{}/tags", repo_name)
                    }
                } else {
                    String::new()
                }
            } else {
                format!("https://hub.docker.com/_/{}/tags", registry_image)
            };

            Some(ContainerInfo {
                id: c.id.as_deref().unwrap_or("").chars().take(12).collect(),
                name,
                image: image_name,
                image_tag: tag,
                status: c.status.as_deref().unwrap_or("unknown").to_string(),
                state: c.state.as_deref().unwrap_or("unknown").to_string(),
                size_mb: ((c.size_rw.unwrap_or(0) as f64 / 1_048_576.0) * 100.0).round() / 100.0,
                has_update: false,
                compose_project: c
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get(LABEL_COMPOSE_PROJECT).cloned()),
                ports,
                traefik_url,
                registry_url,
            })
        })
        .collect()
}

pub async fn pull_image(docker: &Docker, image: &str) -> bool {
    let stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: image,
            ..Default::default()
        }),
        None,
        None,
    );
    pin_mut!(stream);
    while let Some(item) = stream.next().await {
        if item.is_err() {
            return false;
        }
    }
    true
}

async fn list_containers_h(
    State(cache): State<CachedContainers>,
    State(docker): State<Docker>,
    State(config): State<Config>,
) -> Json<Vec<ContainerInfo>> {
    let cached = cache.read().await;
    if let Some(containers) = cached.as_ref() {
        Json(containers.clone())
    } else {
        drop(cached);
        Json(fetch_containers(&docker, &config.allowed_containers).await)
    }
}

async fn inspect_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<ContainerInspectResponse>, AppError> {
    let resp = docker
        .inspect_container(&name, None::<InspectContainerOptions>)
        .await
        .map_err(|e| AppError::NotFound(format!("Container '{}': {}", name, e)))?;
    let ports = resp
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
        .map(|port_map| {
            port_map
                .iter()
                .filter_map(|(key, bindings)| {
                    let parts: Vec<&str> = key.split('/').collect();
                    let private_port: u16 = parts.first().and_then(|p| p.parse().ok())?;
                    let proto = parts.get(1).copied().unwrap_or("tcp").to_string();
                    let public_port = bindings
                        .as_ref()
                        .and_then(|b| b.first())
                        .and_then(|b| b.host_port.as_ref())
                        .and_then(|p| p.parse().ok());
                    Some(PortInfo {
                        private_port,
                        public_port,
                        r#type: proto,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mounts = resp
        .mounts
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|mp| MountInfo {
                    source: mp.source.clone().unwrap_or_default(),
                    destination: mp.destination.clone().unwrap_or_default(),
                    mode: mp.mode.clone().unwrap_or_default(),
                    rw: mp.rw.unwrap_or(false),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let env = resp
        .config
        .as_ref()
        .and_then(|c| c.env.clone())
        .unwrap_or_default();
    let networks = resp
        .network_settings
        .as_ref()
        .and_then(|ns| ns.networks.as_ref())
        .map(|nets| {
            nets.iter()
                .map(|(net_name, settings)| ContainerNetworkInfo {
                    name: net_name.clone(),
                    ip_address: settings.ip_address.clone().unwrap_or_default(),
                    gateway: settings.gateway.clone().unwrap_or_default(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let labels = resp
        .config
        .as_ref()
        .and_then(|c| c.labels.clone())
        .unwrap_or_default();
    let restart_policy = resp
        .host_config
        .as_ref()
        .and_then(|hc| hc.restart_policy.as_ref())
        .and_then(|rp| rp.name.as_ref().map(|n| n.to_string()))
        .unwrap_or_default();
    let health = resp
        .state
        .as_ref()
        .and_then(|s| s.health.as_ref())
        .and_then(|h| h.status.as_ref().map(|s| s.to_string()));
    let state_str = resp
        .state
        .as_ref()
        .and_then(|s| s.status.as_ref().map(|s| s.to_string()))
        .unwrap_or_default();
    let id = resp.id.unwrap_or_default();
    let name_out = strip_name(&resp.name.unwrap_or_default());
    let image = resp.image.unwrap_or_default();
    let created = resp.created.unwrap_or_default();
    Ok(Json(ContainerInspectResponse {
        id,
        name: name_out,
        image,
        created,
        state: state_str.clone(),
        status: state_str,
        ports,
        mounts,
        env,
        networks,
        labels,
        restart_policy,
        health,
    }))
}

async fn start_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    docker
        .start_container(&name, None::<StartContainerOptions<String>>)
        .await
        .map_err(|e| AppError::NotFound(format!("start_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn stop_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    docker
        .stop_container(&name, None::<StopContainerOptions>)
        .await
        .map_err(|e| AppError::NotFound(format!("stop_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn restart_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    docker
        .restart_container(&name, None::<RestartContainerOptions>)
        .await
        .map_err(|e| AppError::NotFound(format!("restart_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn remove_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    docker
        .remove_container(&name, Some(options))
        .await
        .map_err(|e| AppError::NotFound(format!("remove_container '{}': {}", name, e)))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

async fn config_handler(
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<PublicConfig> {
    let s = settings.lock().await;
    let tg_token = s.telegram_token.as_ref().or(config.telegram_token.as_ref());
    let tg_chat_id = s
        .telegram_chat_id
        .clone()
        .or_else(|| config.telegram_chat_id.clone());
    let mx_homeserver = s
        .matrix_homeserver
        .clone()
        .or_else(|| config.matrix_homeserver.clone());
    let mx_token = s.matrix_token.as_ref().or(config.matrix_token.as_ref());
    let mx_room = s.matrix_room.clone().or_else(|| config.matrix_room.clone());
    Json(PublicConfig {
        oidc_configured: true,
        port: config.port(),
        auto_update_enabled: s
            .auto_update_enabled
            .unwrap_or_else(|| config.auto_update()),
        auto_update_interval_hours: s
            .auto_update_interval_hours
            .unwrap_or_else(|| config.auto_update_interval()),
        telegram_configured: tg_token.is_some(),
        telegram_token_set: tg_token.is_some(),
        telegram_chat_id: tg_chat_id,
        matrix_configured: mx_homeserver.is_some(),
        matrix_token_set: mx_token.is_some(),
        matrix_homeserver: mx_homeserver,
        matrix_room: mx_room,
        allowed_containers: config.allowed_containers.clone(),
    })
}

async fn update_config_h(
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
    Json(body): Json<UpdateSettingsReq>,
) -> Json<PublicConfig> {
    {
        let mut s = settings.lock().await;
        if let Some(v) = body.auto_update_enabled {
            s.auto_update_enabled = Some(v);
        }
        if let Some(v) = body.auto_update_interval_hours {
            s.auto_update_interval_hours = Some(v);
        }
        if let Some(v) = body.telegram_token {
            if v.is_empty() {
                s.telegram_token = None;
            } else {
                s.telegram_token = Some(v);
            }
        }
        if let Some(v) = body.telegram_chat_id {
            if v.is_empty() {
                s.telegram_chat_id = None;
            } else {
                s.telegram_chat_id = Some(v);
            }
        }
        if let Some(v) = body.matrix_homeserver {
            if v.is_empty() {
                s.matrix_homeserver = None;
            } else {
                s.matrix_homeserver = Some(v);
            }
        }
        if let Some(v) = body.matrix_token {
            if v.is_empty() {
                s.matrix_token = None;
            } else {
                s.matrix_token = Some(v);
            }
        }
        if let Some(v) = body.matrix_room {
            if v.is_empty() {
                s.matrix_room = None;
            } else {
                s.matrix_room = Some(v);
            }
        }
        json_writer().save(FILE_SETTINGS, &*s).await;
    }
    config_handler(State(config), State(settings)).await
}

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
    crate::workers::json_writer()
        .save(FILE_UPDATES_HISTORY, &*hist)
        .await;
    Json(serde_json::json!({"status": "cleared"}))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/containers", get(list_containers_h))
        .route("/api/containers/{name}/inspect", get(inspect_container_h))
        .route("/api/containers/{name}/start", post(start_container_h))
        .route("/api/containers/{name}/stop", post(stop_container_h))
        .route("/api/containers/{name}/restart", post(restart_container_h))
        .route("/api/containers/{name}/remove", post(remove_container_h))
        .route("/api/config", get(config_handler).put(update_config_h))
        .route("/api/history", get(get_history_h).delete(delete_history_h))
}
