use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
        RemoveContainerOptions, RestartContainerOptions, StartContainerOptions,
        StopContainerOptions,
    },
    image::{CreateImageOptions, RemoveImageOptions, TagImageOptions},
    Docker,
};
use futures::{pin_mut, StreamExt};
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;

use crate::db::DbPool;
use crate::models::*;
use crate::state::AppState;
use crate::updates::health_guard::{is_stable_running, runtime_health, RuntimeHealth};
use crate::workers::CachedContainers;

pub async fn find_container_by_name(
    docker: &Docker,
    name: &str,
) -> Result<bollard::models::ContainerSummary, AppError> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await?;
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

/// Recupera la lista de contenedores enriquecida con el estado persistido en
/// la base de datos.
///
/// El resultado depende de la BD (flags `has_update`, timestamps y digests
/// persistidos), por lo que un fallo de pool o de `Mutex` de conexión se
/// propaga como [`AppError`] en lugar de degradar silenciosamente a `[]`. Los
/// call-sites best-effort (workers, refrescos de estado) deciden entonces si
/// registran el fallo y continúan.
pub async fn fetch_containers(
    docker: &Docker,
    allowed: &Option<Vec<String>>,
    db_pool: &DbPool,
) -> Result<Vec<ContainerInfo>, AppError> {
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

    // Step 2: Read persisted has_update from DB
    let has_update_map: std::collections::HashMap<String, bool> = {
        let conn = db_pool.get().await.map_err(AppError::from_pool)?;
        let guard = conn.lock().map_err(AppError::from_pool)?;
        let mut stmt = guard
            .prepare("SELECT name, has_update FROM containers")
            .map_err(AppError::from_pool)?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let hu: i32 = row.get(1)?;
                Ok((name, hu != 0))
            })
            .map_err(AppError::from_pool)?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Step 2b: Read persisted last_check / next_check from DB
    let check_times_map = {
        let conn = db_pool.get().await.map_err(AppError::from_pool)?;
        let guard = conn.lock().map_err(AppError::from_pool)?;
        let mut stmt = guard
            .prepare(
                "SELECT name, last_check, next_check FROM containers WHERE last_check IS NOT NULL",
            )
            .map_err(AppError::from_pool)?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let lc: Option<String> = row.get(1)?;
                let nc: Option<String> = row.get(2)?;
                Ok((name, (lc, nc)))
            })
            .map_err(AppError::from_pool)?;
        let map: std::collections::HashMap<_, _> = rows.filter_map(|r| r.ok()).collect();
        map
    };

    // Step 2c: Read persisted last_remote_digest from DB
    let last_remote_digest_map: std::collections::HashMap<String, String> = {
        let conn = db_pool.get().await.map_err(AppError::from_pool)?;
        let guard = conn.lock().map_err(AppError::from_pool)?;
        let mut stmt = guard
            .prepare(
                "SELECT name, last_remote_digest FROM containers WHERE last_remote_digest != ''",
            )
            .map_err(AppError::from_pool)?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let digest: String = row.get(1)?;
                Ok((name, digest))
            })
            .map_err(AppError::from_pool)?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Step 2c-bis: Read persisted last_pulled_digest (local image cache) from DB
    let last_pulled_digest_map: std::collections::HashMap<String, String> = {
        let conn = db_pool.get().await.map_err(AppError::from_pool)?;
        let guard = conn.lock().map_err(AppError::from_pool)?;
        crate::db::load_last_pulled_digest_map(&guard)
    };

    // Step 3: Build ContainerInfo list (now all image names are resolved)
    Ok(containers
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
                name: name.clone(),
                image: image_name,
                image_tag: tag,
                status: c.status.as_deref().unwrap_or("unknown").to_string(),
                state: c.state.as_deref().unwrap_or("unknown").to_string(),
                size_mb: ((c.size_rw.unwrap_or(0) as f64 / 1_048_576.0) * 100.0).round() / 100.0,
                has_update: *has_update_map.get(&name).unwrap_or(&false),
                updating: false,
                compose_project: c
                    .labels
                    .as_ref()
                    .and_then(|labels| labels.get(LABEL_COMPOSE_PROJECT).cloned()),
                ports,
                traefik_url,
                registry_url,
                last_check: check_times_map.get(&name).and_then(|(lc, _)| lc.clone()),
                next_check: check_times_map.get(&name).and_then(|(_, nc)| nc.clone()),
                last_remote_digest: last_remote_digest_map
                    .get(&name)
                    .cloned()
                    .unwrap_or_default(),
                last_pulled_digest: last_pulled_digest_map
                    .get(&name)
                    .cloned()
                    .unwrap_or_default(),
            })
        })
        .collect())
}

/// Remove an old image by its digest/ID after a successful update.
/// Only succeeds if no container is still using the image.
pub async fn remove_old_image(docker: &Docker, old_image_id: &str) {
    if old_image_id.is_empty() {
        return;
    }
    match docker
        .remove_image(
            old_image_id,
            Some(RemoveImageOptions {
                force: false,
                noprune: false,
            }),
            None,
        )
        .await
    {
        Ok(_) => {
            tracing::info!(
                "remove_old_image: imagen antigua {} eliminada",
                crate::updates::digest::short_digest(old_image_id)
            );
        }
        Err(e) => {
            tracing::debug!(
                "remove_old_image: no se pudo eliminar {} (en uso?): {}",
                crate::updates::digest::short_digest(old_image_id),
                e
            );
        }
    }
}

/// Inspect a container and build a [`RuntimeHealth`] snapshot from its runtime
/// state: `Status`, `RestartCount` and the HEALTHCHECK result (if any).
///
/// `health` is `None` when the image defines no HEALTHCHECK, or when the
/// daemon reports it as disabled (`none`/empty), so the guard knows it must not
/// require a `healthy` result.
pub async fn inspect_runtime_health(
    docker: &Docker,
    name: &str,
) -> Result<RuntimeHealth, AppError> {
    let resp = docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
        .map_err(|e| AppError::Docker(format!("inspect_container '{}': {}", name, e)))?;

    let state = resp
        .state
        .as_ref()
        .and_then(|s| s.status.as_ref())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let restart_count = resp.restart_count.unwrap_or(0).max(0) as u64;
    let health = resp
        .state
        .as_ref()
        .and_then(|s| s.health.as_ref())
        .and_then(|h| h.status.as_ref())
        .map(|h| h.to_string())
        .filter(|h| !h.is_empty() && h != "none");

    Ok(runtime_health(&state, restart_count, health.as_deref()))
}

/// Verify that a container is *stably* healthy across a time window.
///
/// Takes ~4 samples spread across `window_secs` (one every `window_secs / 4`)
/// and delegates the decision to
/// [`is_stable_running`](crate::updates::health_guard::is_stable_running):
/// `running` in every sample, `RestartCount` not increasing, and `healthy` at
/// the end when the image defines a HEALTHCHECK.
pub async fn verify_stable_running(
    docker: &Docker,
    name: &str,
    window_secs: u64,
) -> Result<bool, AppError> {
    const SAMPLES: u64 = 4;
    let step = (window_secs / SAMPLES).max(1);
    let mut samples = Vec::with_capacity(SAMPLES as usize);
    for _ in 0..SAMPLES {
        tokio::time::sleep(std::time::Duration::from_secs(step)).await;
        samples.push(inspect_runtime_health(docker, name).await?);
    }
    Ok(is_stable_running(&samples))
}

/// Stop a container by name (terminal `failed-stopped` state of the guard).
pub async fn stop_container(docker: &Docker, name: &str) -> Result<(), AppError> {
    docker
        .stop_container(name, None::<StopContainerOptions>)
        .await
        .map_err(|e| AppError::Docker(format!("stop_container '{}': {}", name, e)))
}

/// Build the `(from_image, tag)` pair used by [`pull_image`].
///
/// `tag_ref` strips any pre-existing `@sha256` pin (self-heals containers
/// pinned by earlier versions) so we never build a double-`@` reference.
/// When a digest is given we pull by digest (empty tag); otherwise we pull by
/// the underlying tag.
fn pull_ref(image: &str, digest: Option<&str>) -> (String, String) {
    let base = crate::updates::digest::tag_ref(image);
    match digest {
        Some(d) => (format!("{}@{}", base, d), String::new()),
        None => {
            let (_, tag) = crate::models::parse_image_tag(base);
            (base.to_string(), tag)
        }
    }
}

pub async fn pull_image(
    docker: &Docker,
    image: &str,
    digest: Option<&str>,
    timeout_secs: u64,
) -> bool {
    // When digest is provided, pull using `image@digest` to bypass Docker tag caching.
    // When digest is None, fall back to tag-based pull.
    let (from_image, tag) = pull_ref(image, digest);
    let platform = crate::models::current_platform();
    tracing::info!(
        "pull_image: descargando '{}' (modo: {}, timeout: {}s)",
        from_image,
        if digest.is_some() {
            "por-digest"
        } else {
            "por-tag"
        },
        timeout_secs
    );
    let stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: from_image.clone(),
            tag,
            platform,
            ..Default::default()
        }),
        None,
        None,
    );
    pin_mut!(stream);
    let timeout_dur = std::time::Duration::from_secs(timeout_secs);
    let timed = tokio::time::timeout(timeout_dur, async {
        while let Some(item) = stream.next().await {
            if let Err(e) = item {
                tracing::error!("pull_image: error de Bollard para '{}': {}", image, e);
                return false;
            }
        }
        true
    });
    match timed.await {
        Ok(result) => {
            if !result {
                tracing::error!("pull_image: error durante la descarga de '{}'", image);
                return false;
            }
            tracing::info!("pull_image: descarga completada para '{}'", image);
            // After a digest pull, retag `repo:tag` so the subsequent recreate
            // (which uses the clean tag reference) picks up the freshly
            // downloaded content instead of a cached tag.
            if let Some(d) = digest {
                let clean = crate::updates::digest::tag_ref(image);
                let (repo, tag) = crate::models::parse_image_tag(clean);
                // Tag from a canonical, tag-less reference (`repo@sha256:<d>`)
                // rather than from `repo:tag@sha256:<d>`, which is not a stable
                // source across registries.
                let source = format!("{}@{}", repo, d);
                let opts = TagImageOptions { repo, tag };
                if let Err(e) = docker.tag_image(source.as_str(), Some(opts)).await {
                    tracing::error!(
                        "pull_image: falló el retag de '{}' tras pull por digest: {}",
                        clean,
                        e
                    );
                    return false;
                }
                tracing::info!("pull_image: retag completado '{}'", clean);
            }
            true
        }
        Err(_) => {
            tracing::error!(
                "pull_image: timeout después de {}s para '{}'",
                timeout_secs,
                image
            );
            false
        }
    }
}

/// `GET /api/containers` — listado paginado y filtrado de la flota.
///
/// Acepta `page`, `page_size`, `q`, `state` y `has_update` (ver
/// [`ContainerListQuery`]). Devuelve `{ items, total, page, page_size }`, con
/// `total` calculado tras filtrar. Parámetros inválidos → HTTP 400.
async fn list_containers_h(
    State(cache): State<CachedContainers>,
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    Query(query): Query<ContainerListQuery>,
) -> Result<Json<PagedContainers>, AppError> {
    let params = query.normalize()?;

    // La lista es la respuesta: si hay que leerla de la BD y esta falla, se
    // devuelve `AppError` (HTTP 500) en lugar de un `200 []` engañoso.
    let containers = {
        let cached = cache.read().await;
        match cached.as_ref() {
            Some(containers) => containers.clone(),
            None => {
                drop(cached);
                fetch_containers(&docker, &None, &db_pool).await?
            }
        }
    };

    Ok(Json(paginate_containers(&containers, &params)))
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

async fn logs_container_h(
    State(docker): State<Docker>,
    Path(name): Path<String>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let options = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: "100".to_string(),
        ..Default::default()
    };

    let stream = docker.logs(&name, Some(options));
    let mut stream = Box::pin(stream);

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    tokio::spawn(async move {
        while let Some(item) = stream.next().await {
            match item {
                Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) => {
                    let text = String::from_utf8_lossy(&message).to_string();
                    if tx
                        .send(Ok(Event::default().event("log").data(text)))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    let _ = tx
                        .send(Ok(Event::default()
                            .event("error")
                            .data(format!("Docker error: {}", e))))
                        .await;
                    break;
                }
            }
        }
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/containers", get(list_containers_h))
        .route("/api/containers/{name}/inspect", get(inspect_container_h))
        .route("/api/containers/{name}/start", post(start_container_h))
        .route("/api/containers/{name}/stop", post(stop_container_h))
        .route("/api/containers/{name}/restart", post(restart_container_h))
        .route("/api/containers/{name}/remove", post(remove_container_h))
        .route("/api/containers/{name}/logs", get(logs_container_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;
    use crate::test_support::{podman_client, FixtureContainer};

    // ── fetch_containers ─────────────────────────────────────

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_fetch_containers_returns_list() {
        let docker = podman_client();
        let pool = test_pool();
        let f = FixtureContainer::run("alloy-it").await;
        let result = fetch_containers(&docker, &None, &pool)
            .await
            .expect("fetch_containers should succeed");
        assert!(!result.is_empty(), "Should have at least one container");
        assert!(
            result.iter().any(|c| c.name == f.name()),
            "Fixture '{}' should be in the list",
            f.name()
        );
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_fetch_containers_structure() {
        let docker = podman_client();
        let pool = test_pool();
        let f = FixtureContainer::run("alloy-it").await;
        let containers = fetch_containers(&docker, &None, &pool)
            .await
            .expect("fetch_containers should succeed");
        let c = containers
            .iter()
            .find(|c| c.name == f.name())
            .expect("Fixture 'alloy-it' should be running");

        assert!(!c.id.is_empty(), "Container should have an ID");
        assert!(!c.image.is_empty(), "Container should have an image");
        assert!(!c.image_tag.is_empty(), "Container should have a tag");
        assert_eq!(c.state, "running");
        assert!(
            !c.ports.is_empty() || c.traefik_url.is_some() || !c.registry_url.is_empty(),
            "Container should have ports, traefik URL or registry URL"
        );
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_fetch_containers_fields() {
        let docker = podman_client();
        let pool = test_pool();
        let f = FixtureContainer::run("alloy-it").await;
        let containers = fetch_containers(&docker, &None, &pool)
            .await
            .expect("fetch_containers should succeed");
        for c in &containers {
            // Every container must have these basic fields
            assert!(!c.id.is_empty(), "ID should not be empty for {}", c.name);
            assert!(!c.name.is_empty(), "Name should not be empty");
            assert!(
                !c.image.is_empty(),
                "Image should not be empty for {}",
                c.name
            );
            // State must be one of the Docker states
            assert!(
                [
                    "running",
                    "exited",
                    "paused",
                    "created",
                    "restarting",
                    "removing",
                    "dead"
                ]
                .contains(&c.state.as_str()),
                "Invalid state '{}' for container {}",
                c.state,
                c.name
            );
            // size_mb should be a valid number
            assert!(c.size_mb >= 0.0, "Size should be >= 0 for {}", c.name);
        }
        assert!(
            containers.iter().any(|c| c.name == f.name()),
            "Fixture '{}' should be in the list",
            f.name()
        );
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_fetch_containers_with_allowed_filter() {
        let docker = podman_client();
        let pool = test_pool();
        let f1 = FixtureContainer::run("alloy-it").await;
        let f2 = FixtureContainer::run("alloy-it").await;
        let allowed_names = vec![f1.name().to_string(), f2.name().to_string()];
        let allowed = Some(allowed_names.clone());
        let containers = fetch_containers(&docker, &allowed, &pool)
            .await
            .expect("fetch_containers should succeed");
        for c in &containers {
            assert!(
                allowed_names.contains(&c.name),
                "Filtered container '{}' should not appear",
                c.name
            );
        }
        for name in &allowed_names {
            assert!(
                containers.iter().any(|c| c.name == *name),
                "Fixture '{name}' should appear in the filtered list"
            );
        }
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_fetch_containers_with_empty_allowed() {
        let docker = podman_client();
        let pool = test_pool();
        let allowed = Some(Vec::new());
        let containers = fetch_containers(&docker, &allowed, &pool)
            .await
            .expect("fetch_containers should succeed");
        assert!(
            containers.is_empty(),
            "Empty allowed list should return nothing"
        );
    }

    // ── find_container_by_name ───────────────────────────────

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_find_container_by_name_found() {
        let docker = podman_client();
        let f = FixtureContainer::run("alloy-it").await;
        let container = find_container_by_name(&docker, f.name())
            .await
            .expect("Fixture should be found");
        let name = container
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| strip_name(n));
        assert_eq!(name.as_deref(), Some(f.name()));
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_find_container_by_name_not_found() {
        let docker = podman_client();
        let result = find_container_by_name(&docker, "this-container-does-not-exist-xyz").await;
        assert!(result.is_err());
        match result {
            Err(AppError::NotFound(_)) => {} // expected
            _ => panic!("Expected NotFound error"),
        }
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_find_container_by_name_second_fixture() {
        let docker = podman_client();
        let f1 = FixtureContainer::run("alloy-it").await;
        let f2 = FixtureContainer::run("alloy-it").await;
        // Dos fixtures con nombres distintos coexisten sin colisión.
        assert_ne!(f1.name(), f2.name());

        let container = find_container_by_name(&docker, f2.name())
            .await
            .expect("Fixture should be found");
        let name = container
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| strip_name(n));
        assert_eq!(name.as_deref(), Some(f2.name()));
    }

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_find_container_by_name_fixture_coexists() {
        let docker = podman_client();
        let f1 = FixtureContainer::run("alloy-it").await;
        let f2 = FixtureContainer::run("alloy-it").await;

        let found1 = find_container_by_name(&docker, f1.name())
            .await
            .expect("Fixture 1 should be found");
        let found2 = find_container_by_name(&docker, f2.name())
            .await
            .expect("Fixture 2 should be found");
        let name1 = found1
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| strip_name(n));
        let name2 = found2
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| strip_name(n));

        assert_eq!(name1.as_deref(), Some(f1.name()));
        assert_eq!(name2.as_deref(), Some(f2.name()));
    }

    // ── Port parsing ─────────────────────────────────────────

    #[ignore = "requiere daemon de contenedores y red (pull de la imagen de fixture alpine)"]
    #[tokio::test]
    async fn test_integration_fetch_containers_ports() {
        let docker = podman_client();
        let pool = test_pool();
        let f = FixtureContainer::run("alloy-it").await;
        let containers = fetch_containers(&docker, &None, &pool)
            .await
            .expect("fetch_containers should succeed");

        assert!(
            containers
                .iter()
                .any(|c| c.name == f.name() && !c.ports.is_empty()),
            "Fixture '{}' should expose a published port",
            f.name()
        );

        // Check that containers with ports have valid port strings
        for c in &containers {
            for port in &c.ports {
                // Each port should follow the format "ip:public:private" or be empty
                if !port.is_empty() {
                    let parts: Vec<&str> = port.split(':').collect();
                    assert!(parts.len() >= 2, "Invalid port format: {}", port);
                }
            }
        }
    }

    // ── listado paginado (container-list-pagination) ──────────

    /// Contenedor mínimo para los tests de router del listado.
    fn make_listed_container(
        name: &str,
        state: &str,
        has_update: bool,
        project: Option<&str>,
    ) -> ContainerInfo {
        ContainerInfo {
            id: name.to_string(),
            name: name.to_string(),
            image: format!("{name}:latest"),
            image_tag: "latest".into(),
            size_mb: 1.0,
            status: state.to_string(),
            state: state.to_string(),
            has_update,
            updating: false,
            compose_project: project.map(str::to_string),
            ports: vec![],
            traefik_url: None,
            registry_url: String::new(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }
    }

    /// Router con el listado montado y la cache ya poblada (sin tocar el daemon).
    async fn list_app(containers: Vec<ContainerInfo>) -> Router {
        let state =
            crate::test_support::test_app_state(crate::config::Config::default(), test_pool());
        *state.cached_containers.write().await = Some(containers);
        routes().with_state(state)
    }

    async fn get_json(app: Router, uri: &str) -> (axum::http::StatusCode, serde_json::Value) {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("oneshot");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn test_list_containers_returns_default_page_of_twentyfive() {
        let containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("c{i:02}"), "running", false, Some("alpha")))
            .collect();
        let (status, json) = get_json(list_app(containers).await, "/api/containers").await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["page"], 1);
        assert_eq!(json["page_size"], 20);
        assert_eq!(json["total"], 25);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(20));
    }

    #[tokio::test]
    async fn test_list_containers_second_page_returns_five() {
        let containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("c{i:02}"), "running", false, Some("alpha")))
            .collect();
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?page=2&page_size=20",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(5));
        assert_eq!(json["total"], 25);
    }

    #[tokio::test]
    async fn test_list_containers_page_beyond_end_is_empty_with_real_total() {
        let containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("c{i:02}"), "running", false, Some("alpha")))
            .collect();
        let (status, json) = get_json(list_app(containers).await, "/api/containers?page=99").await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(0));
        assert_eq!(json["total"], 25);
    }

    #[tokio::test]
    async fn test_list_containers_rejects_page_size_30_with_400() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?page_size=30",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_containers_rejects_page_zero_with_400() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?page=0",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    // ── casos límite de la query ─────────────────────────────

    #[tokio::test]
    async fn test_list_containers_rejects_non_numeric_page_with_400() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?page=abc",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_containers_rejects_non_boolean_has_update_with_400() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?has_update=maybe",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_containers_rejects_negative_page_size_with_400() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?page_size=-1",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    /// `4294967296` no cabe en `u32` → la deserialización falla con 400.
    #[tokio::test]
    async fn test_list_containers_rejects_u32_overflow_page_with_400() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?page=4294967296",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    /// Clave escalar repetida (`?page=1&page=2`): last-wins, no 400.
    #[tokio::test]
    async fn test_list_containers_repeated_scalar_uses_last_value() {
        let containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("c{i:02}"), "running", false, Some("alpha")))
            .collect();
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?page=1&page=2&page_size=20",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["page"], 2, "debe aplicarse el último valor");
        assert_eq!(json["items"].as_array().map(Vec::len), Some(5));
        assert_eq!(json["total"], 25);
    }

    /// Clave desconocida (`?pageSize=50`): se ignora y cae al default.
    #[tokio::test]
    async fn test_list_containers_ignores_unknown_query_keys() {
        let (status, json) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?pageSize=50",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["page_size"], 20, "typo ignorado → default");
    }

    #[tokio::test]
    async fn test_list_containers_filters_by_state_and_has_update() {
        let containers = vec![
            make_listed_container("web", "running", true, None),
            make_listed_container("api", "running", false, None),
            make_listed_container("db", "exited", true, None),
        ];
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?state=running&has_update=true",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["total"], 1);
        assert_eq!(json["items"][0]["name"], "web");
    }

    #[tokio::test]
    async fn test_list_containers_q_searches_name_image_and_project() {
        let containers = vec![
            make_listed_container("web", "running", false, Some("shop")),
            make_listed_container("db", "running", false, Some("blog")),
        ];
        let (status, json) = get_json(list_app(containers).await, "/api/containers?q=BLOG").await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["total"], 1);
        assert_eq!(json["items"][0]["name"], "db");
    }

    #[tokio::test]
    async fn test_list_containers_accepts_repeated_state_params() {
        let containers = vec![
            make_listed_container("web", "running", false, None),
            make_listed_container("db", "exited", false, None),
            make_listed_container("cache", "paused", false, None),
        ];
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?state=running&state=exited",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["total"], 2);
        let names: Vec<&str> = json["items"]
            .as_array()
            .expect("items array")
            .iter()
            .filter_map(|v| v["name"].as_str())
            .collect();
        assert!(
            names.contains(&"web") && names.contains(&"db"),
            "deben aparecer running y exited: {names:?}"
        );
        assert!(
            !names.contains(&"cache"),
            "no debe aparecer paused: {names:?}"
        );
    }

    // ── modo all=true (listado completo) ─────────────────────
    //
    // `all=true` devuelve TODA la flota filtrada en una sola respuesta: `items`
    // incluye todos los contenedores, `total` es su número y `page`/`page_size`
    // reflejan el conjunto completo (`page=1`, `page_size=items.len()`). Los
    // parámetros `page`/`page_size` de la petición se ignoran (no se validan).

    /// `all=true` con una flota mayor que la página por defecto devuelve los 120.
    #[tokio::test]
    async fn test_list_containers_all_returns_every_container() {
        let containers: Vec<ContainerInfo> = (0..120)
            .map(|i| make_listed_container(&format!("c{i:03}"), "running", false, Some("alpha")))
            .collect();
        let (status, json) = get_json(list_app(containers).await, "/api/containers?all=true").await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(120));
        assert_eq!(json["total"], 120);
        assert_eq!(json["page"], 1);
        assert_eq!(json["page_size"], 120);
    }

    /// `all=true` aplica los filtros y devuelve TODOS los que pasan el filtro,
    /// no solo la primera página.
    #[tokio::test]
    async fn test_list_containers_all_applies_state_filter() {
        let mut containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("run{i:03}"), "running", false, None))
            .collect();
        containers.extend(
            (0..10).map(|i| make_listed_container(&format!("stop{i:03}"), "exited", false, None)),
        );
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?all=true&state=running",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(25));
        assert_eq!(json["total"], 25);
    }

    /// `all=true&page_size=30`: `page_size` se ignora, no se valida → `200` con
    /// todos los items (no `400`).
    #[tokio::test]
    async fn test_list_containers_all_ignores_invalid_page_size() {
        let containers: Vec<ContainerInfo> = (0..3)
            .map(|i| make_listed_container(&format!("c{i}"), "running", false, None))
            .collect();
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?all=true&page_size=30",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(3));
        assert_eq!(json["total"], 3);
    }

    /// `all=true` también ignora `page`: no recorta a partir de la página 2.
    #[tokio::test]
    async fn test_list_containers_all_ignores_page() {
        let containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("c{i:03}"), "running", false, None))
            .collect();
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?all=true&page=2&page_size=10",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(25));
        assert_eq!(json["total"], 25);
    }

    /// `all=true` combinado con el filtro `q` devuelve todos los coincidentes.
    #[tokio::test]
    async fn test_list_containers_all_applies_q_filter() {
        let mut containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("web{i:03}"), "running", false, Some("alpha")))
            .collect();
        containers.extend(
            (0..5).map(|i| {
                make_listed_container(&format!("db{i:03}"), "running", false, Some("beta"))
            }),
        );
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?all=true&q=alpha",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(25));
        assert_eq!(json["total"], 25);
    }

    /// Regresión: `all=false` (o ausente) mantiene la paginación intacta.
    #[tokio::test]
    async fn test_list_containers_all_false_keeps_pagination() {
        let containers: Vec<ContainerInfo> = (0..25)
            .map(|i| make_listed_container(&format!("c{i:03}"), "running", false, None))
            .collect();
        let (status, json) = get_json(
            list_app(containers).await,
            "/api/containers?all=false&page=1&page_size=20",
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(20));
        assert_eq!(json["total"], 25);
        assert_eq!(json["page"], 1);
        assert_eq!(json["page_size"], 20);
    }

    /// Regresión: sin `all`, `page_size=30` sigue siendo un `400`.
    #[tokio::test]
    async fn test_list_containers_without_all_still_rejects_page_size_30() {
        let (status, _) = get_json(
            list_app(vec![make_listed_container("web", "running", false, None)]).await,
            "/api/containers?all=false&page_size=30",
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    // ── strip_name ───────────────────────────────────────────

    #[test]
    fn test_strip_name_slash() {
        assert_eq!(strip_name("/test_container"), "test_container");
    }

    #[test]
    fn test_strip_name_multi_slash() {
        assert_eq!(strip_name("///test"), "test");
    }

    #[test]
    fn test_strip_name_no_slash() {
        assert_eq!(strip_name("plain_name"), "plain_name");
    }

    #[test]
    fn test_strip_name_empty() {
        assert_eq!(strip_name(""), "");
    }

    // ── pull_ref (self-heal of pinned references) ────────────

    #[test]
    fn test_pull_ref_by_digest_strips_existing_pin() {
        let (from_image, tag) = pull_ref("gitea/gitea:1.27.3@sha256:old", Some("sha256:new"));
        assert_eq!(from_image, "gitea/gitea:1.27.3@sha256:new");
        assert!(tag.is_empty());
    }

    #[test]
    fn test_pull_ref_by_digest_plain_tag() {
        let (from_image, tag) = pull_ref("nginx:alpine", Some("sha256:new"));
        assert_eq!(from_image, "nginx:alpine@sha256:new");
        assert!(tag.is_empty());
    }

    #[test]
    fn test_pull_ref_by_tag_strips_existing_pin() {
        let (from_image, tag) = pull_ref("gitea/gitea:1.27.3@sha256:old", None);
        assert_eq!(from_image, "gitea/gitea:1.27.3");
        assert_eq!(tag, "1.27.3");
    }

    #[test]
    fn test_pull_ref_by_tag_defaults_latest() {
        let (from_image, tag) = pull_ref("nginx", None);
        assert_eq!(from_image, "nginx");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_pull_ref_registry_with_port_and_digest() {
        let (from_image, tag) = pull_ref(
            "registry.example.com:5000/myimage:v2@sha256:abc123",
            Some("sha256:new"),
        );
        assert_eq!(
            from_image,
            "registry.example.com:5000/myimage:v2@sha256:new"
        );
        assert!(tag.is_empty());
    }

    #[test]
    fn test_pull_ref_registry_with_port_by_tag() {
        let (from_image, tag) = pull_ref("registry.example.com:5000/myimage:v2", None);
        assert_eq!(from_image, "registry.example.com:5000/myimage:v2");
        assert_eq!(tag, "v2");
    }
}
