use axum::{
    extract::{Path, State},
    response::Json,
};
use bollard::{
    container::{
        Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
        NetworkingConfig, RemoveContainerOptions, RenameContainerOptions, RestartContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    image::{PruneImagesOptions, RemoveImageOptions, TagImageOptions},
    Docker,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, Notify};

use crate::containers::{fetch_containers, find_container_by_name, pull_image};
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::check_remote_digest_with_docker;
use bollard::models::ImagePruneResponse;

/// Cache progress for the long-polling state endpoint.
/// Updates the BatchProgress struct and notifies waiting handlers.
#[allow(clippy::too_many_arguments)]
async fn update_progress(
    progress_cache: &Arc<Mutex<BatchProgress>>,
    state_notify: &Arc<Notify>,
    total: u32,
    checked: u32,
    updated: u32,
    errors: u32,
    checking: String,
) {
    let mut cache = progress_cache.lock().await;
    cache.total = total;
    cache.checked = checked;
    cache.updated = updated;
    cache.errors = errors;
    cache.checking = checking;
    tracing::info!(
        "[update_progress] total={} checked={} updated={} errors={} checking={}",
        cache.total,
        cache.checked,
        cache.updated,
        cache.errors,
        cache.checking
    );
    state_notify.notify_waiters();
}

/// Image reference written to `Config.Image` when recreating a container.
/// Always the clean `repo:tag` form, never a `repo:tag@sha256:...` pin.
pub(crate) fn recreate_image_ref(image_full: &str) -> String {
    crate::updates::digest::tag_ref(image_full).to_string()
}

/// Whether the initial `last_remote_digest` should be seeded for a container.
/// Seed only when no digest is stored yet and there is no pending update, so
/// the next cycle compares the running image against the remote.
fn should_seed_last_remote_digest(has_stored: bool, has_update: bool) -> bool {
    !has_stored && !has_update
}

/// Whether a successful policy action means the container was actually
/// recreated (so the freshly pulled image is now in use). Only then may
/// `has_update` be cleared and `last_remote_digest` advance.
///
/// `None` and `Pull` update the local image cache but leave the running
/// container on its old image, so they MUST NOT advance the digest.
pub(crate) fn should_advance_digest(action: &UpdateAction) -> bool {
    matches!(
        action,
        UpdateAction::PullRestart | UpdateAction::PullRestartStack
    )
}

/// Whether a `Pull` action can skip the download because the remote config
/// digest is already the one present in the local image cache.
///
/// Returns `true` only for `UpdateAction::Pull` when both digests are present,
/// non-empty and equal. Recreate actions (`PullRestart`/`PullRestartStack`)
/// always pull because they redeploy the container.
pub(crate) fn should_skip_pull(
    action: &UpdateAction,
    remote: Option<&str>,
    last_pulled: Option<&str>,
) -> bool {
    let (Some(remote), Some(last_pulled)) = (remote, last_pulled) else {
        return false;
    };
    *action == UpdateAction::Pull && !remote.is_empty() && remote == last_pulled
}

/// Persist `last_pulled_digest` for a container after a successful pull.
/// Does nothing when the digest is missing/empty, the DB is unavailable, or the
/// value is already stored (avoids redundant writes).
async fn persist_last_pulled_digest(db_pool: &DbPool, name: &str, digest: Option<&str>) {
    let Some(digest) = digest.filter(|d| !d.is_empty()) else {
        return;
    };
    if let Ok(conn) = db_pool.get().await {
        match conn.lock() {
            Ok(guard) => {
                if db::get_container_last_pulled_digest(&guard, name).as_deref() == Some(digest) {
                    return;
                }
                if let Err(e) = db::update_container_last_pulled_digest(&guard, name, digest) {
                    tracing::error!(
                        "persist_last_pulled_digest: error almacenando '{}': {}",
                        name,
                        e
                    );
                }
            }
            Err(e) => tracing::error!(
                "persist_last_pulled_digest: mutex envenenado para '{}': {}",
                name,
                e
            ),
        }
    } else {
        tracing::error!(
            "persist_last_pulled_digest: no se pudo obtener conexión DB para '{}'",
            name
        );
    }
}

/// Recreate a container by stopping, backing up, inspecting, creating with new image,
/// starting, and cleaning up the backup. On failure, attempts rollback.
///
/// `_digest` is accepted for call-site symmetry but is no longer used to build
/// `Config.Image`: the container is always recreated by the clean `repo:tag`
/// reference (the pull-by-digest + retag step already points `repo:tag` at the
/// freshly downloaded content).
pub(crate) async fn recreate_container(
    docker: &Docker,
    name: &str,
    cid: &str,
    image_full: &str,
    _digest: Option<&str>,
) -> Result<(), String> {
    // 1. Stop the old container
    docker
        .stop_container(cid, None::<StopContainerOptions>)
        .await
        .map_err(|e| format!("stop failed: {}", e))?;

    // 2. Rename old container as backup
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let backup_name = format!("{}-old-{}", name, ts);
    docker
        .rename_container(
            cid,
            RenameContainerOptions {
                name: backup_name.clone(),
            },
        )
        .await
        .map_err(|e| format!("rename failed: {}", e))?;

    // 3. Inspect old container to get config
    let inspect = docker
        .inspect_container(&backup_name, None::<InspectContainerOptions>)
        .await
        .map_err(|e| format!("inspect failed: {}", e))?;

    // 4. Build new config from old one, modifying the image reference.
    // Always use the clean `repo:tag` form — never persist the `@sha256` pin.
    let mut container_config = inspect.config.unwrap_or_default();
    container_config.image = Some(recreate_image_ref(image_full));

    // Convert to bollard::container::Config and preserve host/networking config
    let mut config: Config<String> = container_config.into();
    config.host_config = inspect.host_config.clone();

    // Preserve network connections (critical for compose stacks)
    if let Some(networks) = inspect
        .network_settings
        .as_ref()
        .and_then(|ns| ns.networks.clone())
    {
        if !networks.is_empty() {
            config.networking_config = Some(NetworkingConfig {
                endpoints_config: networks,
            });
        }
    }

    // 5. Create new container with same name
    let create_opts = CreateContainerOptions {
        name: name.to_string(),
        platform: None,
    };
    docker
        .create_container(Some(create_opts), config)
        .await
        .map_err(|e| format!("create failed: {}", e))?;

    // 6. Start new container
    if let Err(e) = docker
        .start_container::<String>(name, None::<StartContainerOptions<String>>)
        .await
    {
        // If start fails, attempt rollback: remove the new container,
        // restore backup name, restart old container
        tracing::warn!(
            "recreate_container: start failed for '{}', attempting rollback: {}",
            name,
            e
        );
        let _ = docker
            .remove_container(name, None::<RemoveContainerOptions>)
            .await;
        let _ = docker
            .rename_container(
                &backup_name,
                RenameContainerOptions {
                    name: name.to_string(),
                },
            )
            .await;
        let _ = docker
            .start_container::<String>(name, None::<StartContainerOptions<String>>)
            .await;
        return Err(format!("start failed, rollback attempted: {}", e));
    }

    // 7. Remove old backup container
    let _ = docker
        .remove_container(&backup_name, None::<RemoveContainerOptions>)
        .await;

    Ok(())
}

/// Shared API-only update path used by the stack update handler and the
/// `PullRestartStack` policy branch: pull the image (by manifest digest when
/// available) and recreate the container via the Docker API. No CLI.
///
/// Returns the failure cause so callers can surface it.
pub(crate) async fn pull_and_recreate(
    docker: &Docker,
    name: &str,
    cid: &str,
    image_full: &str,
    manifest_digest: Option<&str>,
    timeout_secs: u64,
) -> Result<(), String> {
    if !pull_image(docker, image_full, manifest_digest, timeout_secs).await {
        return Err(format!("pull de la imagen '{}' falló", image_full));
    }
    recreate_container(docker, name, cid, image_full, manifest_digest)
        .await
        .map_err(|e| format!("recreate de '{}' falló: {}", name, e))
}

struct PendingUpdate {
    name: String,
    image_full: String,
    cid: String,
    image_id: String,
    remote_digest: Option<String>, // config digest (for comparison + DB storage)
    manifest_digest: Option<String>, // manifest digest (for pull + recreate)
    last_pulled_digest: Option<String>, // cached config digest (skip redundant pulls)
    compose_project: Option<String>,
}

pub async fn update_container_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
    Path(name): Path<String>,
) -> Result<Json<UpdateProgress>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image = container.image.as_deref().unwrap_or("");
    let cid = container.id.as_deref().unwrap_or("");

    if image.is_empty() {
        return Err(AppError::BadRequest("container has no image".into()));
    }

    // Verificar digest remoto antes de hacer pull
    let image_id = container.image_id.as_deref().unwrap_or("").to_string();
    let (needs_pull, remote_digest, manifest_digest) =
        match crate::updates::digest::check_remote_digest_with_docker(image, &docker).await {
            Ok((manifest, config, _)) => {
                // Use last_remote_digest from DB if available, fallback to image_id
                let last_remote = {
                    let conn = db_pool.get().await.unwrap();
                    let guard = conn.lock().unwrap();
                    crate::db::get_container_last_remote_digest(&guard, &name)
                        .unwrap_or_else(|| image_id.clone())
                };
                let needs = crate::updates::common::needs_update(&last_remote, &config);
                (needs, config, manifest)
            }
            Err(e) => {
                tracing::warn!(
                    "update_container [{}]: error verificando digest remoto: {}, NO se hará pull",
                    name,
                    e
                );
                return Err(AppError::Internal(format!(
                    "cannot check remote digest: {}",
                    e
                )));
            }
        };

    if !needs_pull {
        return Ok(Json(UpdateProgress {
            container: name,
            status: "already-up-to-date".into(),
            done: true,
            error: None,
            total: 0,
            checked: 0,
            updated: 0,
            errors: 0,
        }));
    }

    let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
    let start_time = std::time::Instant::now();
    tracing::info!(
        "update_container_h: descargando '{}' (timeout: {}s)",
        image,
        pull_timeout
    );
    if !pull_image(&docker, image, Some(&manifest_digest), pull_timeout).await {
        let entry = UpdateHistoryEntry {
            container: name.clone(),
            image: image.to_string(),
            old_digest: image_id.clone(),
            new_digest: remote_digest.clone(),
            timestamp: crate::timezone::now_formatted(),
            status: "error".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
        };
        let mut hist = update_history.lock().await;
        hist.push(entry);
        {
            let conn = db_pool.get().await.unwrap();
            let conn_lock = conn.lock().unwrap();
            let _ = db::append_update_history(&conn_lock, hist.last().unwrap());
            // NOTE: do NOT advance `last_remote_digest` here — the pull failed,
            // so the running container is still on the old image.
        }
        return Err(AppError::Internal("pull failed".into()));
    }
    // Record the digest of the image now present in the local cache.
    persist_last_pulled_digest(&db_pool, &name, Some(&remote_digest)).await;
    match recreate_container(&docker, &name, cid, image, Some(&manifest_digest)).await {
        Ok(_) => {
            tracing::info!("update_container_h: '{}' reiniciado correctamente", name);
            notify_all(&settings, &name, "✅ actualizado y reiniciado").await;
            crate::containers::remove_old_image(&docker, &image_id).await;
            {
                let conn = db_pool.get().await.unwrap();
                let conn_lock = conn.lock().unwrap();
                let _ = db::update_container_has_update(&conn_lock, &name, false);
                let _ = db::update_container_last_remote_digest(&conn_lock, &name, &remote_digest);
            }
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.to_string(),
                old_digest: image_id.clone(),
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "success".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            Ok(Json(UpdateProgress {
                container: name,
                status: "ok".into(),
                done: true,
                error: None,
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
            }))
        }
        Err(e) => {
            tracing::error!("update_container_h: error al reiniciar '{}': {}", name, e);
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.to_string(),
                old_digest: image_id.clone(),
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            Err(AppError::Docker(format!("restart: {}", e)))
        }
    }
}

pub async fn update_all_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
) -> Json<Vec<UpdateProgress>> {
    let mut results = vec![];
    for (name, image, cid, image_id) in crate::workers::docker_list_running(&docker).await {
        // Verificar digest remoto antes de hacer pull
        let (needs_pull, remote_digest, manifest_digest) =
            match crate::updates::digest::check_remote_digest_with_docker(&image, &docker).await {
                Ok((manifest, config, _)) => {
                    let has_update = image_id.as_ref().is_none_or(|local_digest| {
                        let local_short = crate::updates::digest::short_digest(local_digest);
                        let remote_short = crate::updates::digest::short_digest(&config);
                        local_short != remote_short
                    });
                    (has_update, config, manifest)
                }
                Err(e) => {
                    tracing::warn!(
                        "update_all [{}]: error verificando digest remoto: {}, NO se hará pull",
                        name,
                        e
                    );
                    results.push(UpdateProgress {
                        container: name.clone(),
                        status: "error".into(),
                        done: true,
                        error: Some(format!("digest check failed: {}", e)),
                        total: 0,
                        checked: 0,
                        updated: 0,
                        errors: 0,
                    });
                    continue;
                }
            };

        if !needs_pull {
            results.push(UpdateProgress {
                container: name.clone(),
                status: "✅ ya actualizado".into(),
                done: true,
                error: None,
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
            });
            continue;
        }

        let old_digest = image_id.as_deref().unwrap_or("").to_string();
        let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
        let start_time = std::time::Instant::now();
        tracing::info!(
            "update_all_h: descargando '{}' (imagen: {}, timeout: {}s)",
            name,
            image,
            pull_timeout
        );
        if !pull_image(&docker, &image, Some(&manifest_digest), pull_timeout).await {
            tracing::error!("update_all_h: pull FALLÓ para '{}'", name);
            results.push(UpdateProgress {
                container: name.clone(),
                status: "error".into(),
                done: true,
                error: Some("pull failed".into()),
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
            });
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.clone(),
                old_digest,
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            continue;
        }
        tracing::info!("update_all_h: pull OK para '{}', reiniciando...", name);
        // Record the digest of the image now present in the local cache.
        persist_last_pulled_digest(&db_pool, &name, Some(&remote_digest)).await;
        match recreate_container(&docker, &name, &cid, &image, Some(&manifest_digest)).await {
            Ok(_) => {
                tracing::info!("update_all_h: contenedor '{}' recreado correctamente", name);
                notify_all(&settings, &name, "✅ actualizado").await;
                crate::containers::remove_old_image(&docker, &old_digest).await;
                if let Ok(conn) = db_pool.get().await {
                    match conn.lock() {
                        Ok(conn_lock) => {
                            let _ = db::update_container_has_update(&conn_lock, &name, false);
                            // Recreate succeeded → the running image is now the remote
                            // config digest; advance `last_remote_digest`.
                            let _ = db::update_container_last_remote_digest(
                                &conn_lock,
                                &name,
                                &remote_digest,
                            );
                        }
                        Err(e) => tracing::error!(
                            "update_all_h: mutex envenenado al persistir digest de '{}': {}",
                            name,
                            e
                        ),
                    }
                }
                results.push(UpdateProgress {
                    container: name.clone(),
                    status: "ok".into(),
                    done: true,
                    error: None,
                    total: 0,
                    checked: 0,
                    updated: 0,
                    errors: 0,
                });
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest,
                    new_digest: remote_digest.clone(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "success".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            }
            Err(e) => {
                tracing::error!("update_all_h: error al reiniciar '{}': {}", name, e);
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest: old_digest.clone(),
                    new_digest: remote_digest.clone(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());

                results.push(UpdateProgress {
                    container: name,
                    status: "error".into(),
                    done: true,
                    error: Some(e.to_string()),
                    total: 0,
                    checked: 0,
                    updated: 0,
                    errors: 0,
                });
            }
        }
    }
    Json(results)
}

pub async fn check_update_h(
    State(docker): State<Docker>,
    State(db_pool): State<db::DbPool>,
    Path(name): Path<String>,
) -> Result<Json<VersionCompare>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image_full = container.image.as_deref().unwrap_or("");
    tracing::info!("check_update [{}]: imagen={}", name, image_full);
    let (has_update, local_tag, remote_digest, remote_tag, error) = if image_full.is_empty() {
        tracing::warn!("check_update [{}]: contenedor sin imagen", name);
        (None, "unknown".into(), None, None, Some("no image".into()))
    } else {
        let local_tag = crate::updates::digest::parse_image_ref(image_full).tag;
        let (remote_digest, remote_tag, error) =
            match check_remote_digest_with_docker(image_full, &docker).await {
                Ok((_manifest, config, tag)) => (Some(config), Some(tag), None),
                Err(e) => {
                    tracing::warn!(
                        "check_update [{}]: error obteniendo digest remoto: {}",
                        name,
                        e
                    );
                    (None, None, Some(e))
                }
            };
        let has_update = match (&container.image_id, &remote_digest) {
            (Some(local_digest), Some(remote_digest)) => {
                let local_short = crate::updates::digest::short_digest(local_digest);
                let remote_short = crate::updates::digest::short_digest(remote_digest);
                let result = local_short != remote_short;
                tracing::info!(
                    "check_update [{}]: local={} remote={} has_update={}",
                    name,
                    local_short,
                    remote_short,
                    result
                );
                Some(result)
            }
            _ => None,
        };
        (has_update, local_tag, remote_digest, remote_tag, error)
    };
    // Persist has_update to database
    if let Some(hu) = has_update {
        let conn = db_pool.get().await.unwrap();
        let _ = db::update_container_has_update(&conn.lock().unwrap(), &name, hu);
        drop(conn);
    }
    let local_digest = container
        .image_id
        .as_ref()
        .map(|d| crate::updates::digest::short_digest(d));
    Ok(Json(VersionCompare {
        local_tag,
        remote_tag,
        has_update,
        local_digest,
        remote_digest: remote_digest.map(|d| crate::updates::digest::short_digest(&d)),
        changelog_url: None,
        error,
    }))
}

/// Check all containers sequentially, applying policies inline.
/// Returns the container list with has_update flags updated.
/// This replaces the old parallel check + background apply pattern to avoid
/// Docker Hub rate limiting (429 errors).
#[allow(clippy::too_many_arguments)]
async fn check_and_apply_all(
    docker: &Docker,
    db_pool: &DbPool,
    tx: &broadcast::Sender<StateEvent>,
    state_notify: &Arc<Notify>,
    progress_cache: &Arc<Mutex<BatchProgress>>,
    cancel_check: &Arc<AtomicBool>,
    settings: &Arc<Mutex<Settings>>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
) -> Vec<ContainerInfo> {
    let mut containers = fetch_containers(docker, &None, db_pool).await;
    let raw_containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
    tracing::info!(
        "check_and_apply_all: verificando {} contenedores",
        containers.len()
    );

    let check_interval = {
        let s = settings.lock().await;
        s.check_interval_ms.unwrap_or(2000)
    };

    // Pre-build a name→(&ContainerSummary, image_id) map from raw containers
    let raw_map: HashMap<String, (&bollard::models::ContainerSummary, String)> = raw_containers
        .iter()
        .filter_map(|ct| {
            let name = ct
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| crate::models::strip_name(n))?;
            let image_id = ct.image_id.as_deref().unwrap_or("").to_string();
            Some((name, (ct, image_id)))
        })
        .collect();

    // Load last_remote_digest from DB for accurate comparison
    let last_remote_digest_map: HashMap<String, String> = {
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        crate::db::load_last_remote_digest_map(&guard)
    };

    // Load last_pulled_digest from DB (local image cache) to skip redundant pulls
    let last_pulled_digest_map: HashMap<String, String> = match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => crate::db::load_last_pulled_digest_map(&guard),
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    };

    let mut any_success = false;
    let total = containers.len() as u32;
    let mut checked = 0u32;
    let mut updated = 0u32;
    let mut num_errors = 0u32;

    for c in &mut containers {
        let name = c.name.clone();
        let image_full = if c.image_tag.is_empty() {
            c.image.clone()
        } else {
            format!("{}:{}", c.image, c.image_tag)
        };

        if image_full.is_empty() {
            tracing::warn!("check_and_apply_all [{}]: sin imagen, omitiendo", name);
            continue;
        }

        // Get image_id and cid from raw map
        let (raw, image_id) = match raw_map.get(name.as_str()) {
            Some((ct, id)) => (*ct, id.clone()),
            None => {
                tracing::warn!(
                    "check_and_apply_all [{}]: no encontrado en raw_containers, omitiendo",
                    name
                );
                continue;
            }
        };

        tracing::info!(
            "check_and_apply_all [{}]: verificando imagen {}",
            name,
            image_full
        );

        checked += 1;

        // Send progress event so frontend shows live feedback
        update_progress(
            progress_cache,
            state_notify,
            total,
            checked,
            updated,
            num_errors,
            name.clone(),
        )
        .await;

        match check_remote_digest_with_docker(&image_full, docker).await {
            Ok((manifest_digest, config_digest, _)) => {
                // Use last_remote_digest from DB if available, fallback to image_id.
                // This correctly handles non-DockerHub registries where image_id
                // becomes a manifest digest after recreate with image@manifest_digest.
                let local_ref = last_remote_digest_map
                    .get(&name)
                    .map(|s| s.as_str())
                    .unwrap_or(&image_id);
                let has_update = crate::updates::common::needs_update(local_ref, &config_digest);

                tracing::info!(
                    "check_and_apply_all [{}]: local={} remote={} has_update={}",
                    name,
                    crate::updates::digest::short_digest(&image_id),
                    crate::updates::digest::short_digest(&config_digest),
                    has_update,
                );

                // Update has_update in container and DB
                c.has_update = has_update;
                {
                    let conn = db_pool.get().await.unwrap();
                    let conn_lock = conn.lock().unwrap();
                    if let Err(e) = db::update_container_has_update(&conn_lock, &name, has_update) {
                        tracing::error!(
                            "check_and_apply_all: error updating has_update for '{}': {}",
                            name,
                            e
                        );
                    }
                    // `last_remote_digest` represents the image the container is
                    // actually running, so it MUST NOT advance merely because a
                    // newer remote digest was observed. Seed it once (when there
                    // was no stored value and no update pending) so subsequent
                    // cycles have a baseline; the recreate paths persist it after
                    // a successful recreate.
                    if should_seed_last_remote_digest(
                        last_remote_digest_map.contains_key(&name),
                        has_update,
                    ) {
                        if let Err(e) = db::update_container_last_remote_digest(
                            &conn_lock,
                            &name,
                            &config_digest,
                        ) {
                            tracing::error!(
                                "check_and_apply_all: error seeding last_remote_digest for '{}': {}",
                                name,
                                e
                            );
                        }
                    }
                    drop(conn_lock);
                }

                // If has_update and running → apply policy inline
                if has_update && c.state == "running" {
                    let cid = raw.id.as_deref().unwrap_or("").to_string();
                    let compose_project = raw
                        .labels
                        .as_ref()
                        .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                        .cloned();
                    let pending = PendingUpdate {
                        name: name.clone(),
                        image_full: image_full.clone(),
                        cid,
                        image_id,
                        remote_digest: Some(config_digest),
                        manifest_digest: Some(manifest_digest),
                        last_pulled_digest: last_pulled_digest_map.get(&name).cloned(),
                        compose_project,
                    };
                    // Resolve policy for this single container
                    let policies = update_policies.lock().await.clone();
                    let policy_map: HashMap<String, UpdatePolicy> = policies
                        .into_iter()
                        .map(|p| (p.container.clone(), p))
                        .collect();
                    let (default_action, default_cleanup, default_rollback) = {
                        let s = settings.lock().await;
                        (
                            s.default_update_action
                                .clone()
                                .unwrap_or_else(|| "pull-restart".into()),
                            s.default_cleanup_old_image.unwrap_or(false),
                            s.default_rollback_on_failure.unwrap_or(false),
                        )
                    };
                    let policy = policy_map.get(&name).cloned().unwrap_or(UpdatePolicy {
                        container: name.clone(),
                        action: default_action.parse().unwrap_or(UpdateAction::PullRestart),
                        cleanup_old_image: default_cleanup,
                        rollback_on_failure: default_rollback,
                        notify_events: false,
                    });
                    let prev_any_success = any_success;
                    apply_single_policy(
                        docker,
                        settings,
                        state_notify,
                        progress_cache,
                        update_history,
                        db_pool,
                        tx,
                        &pending,
                        &policy,
                        &mut any_success,
                        total,
                        checked,
                        updated,
                        num_errors,
                    )
                    .await;
                    if any_success && !prev_any_success {
                        updated += 1;
                    }
                } else {
                    // No update needed — mark check as done
                    update_progress(
                        progress_cache,
                        state_notify,
                        total,
                        checked,
                        updated,
                        num_errors,
                        name.clone(),
                    )
                    .await;
                }
            }
            Err(e) => {
                num_errors += 1;
                if e.contains("429") {
                    tracing::warn!(
                        "check_and_apply_all [{}]: rate limit (429) fetching digest, saltando",
                        name
                    );
                } else if e.contains("404") {
                    tracing::warn!(
                        "check_and_apply_all [{}]: imagen remota no encontrada (404): {}",
                        name,
                        image_full
                    );
                } else if e.contains("403") {
                    tracing::warn!(
                        "check_and_apply_all [{}]: acceso denegado (403) a: {}",
                        name,
                        image_full
                    );
                } else {
                    tracing::warn!(
                        "check_and_apply_all [{}]: error consultando digest remoto: {}",
                        name,
                        e
                    );
                }
                update_progress(
                    progress_cache,
                    state_notify,
                    total,
                    checked,
                    updated,
                    num_errors,
                    name.clone(),
                )
                .await;
            }
        }

        // Sleep between checks to avoid hammering registries
        if check_interval > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(check_interval)).await;
        }

        // Check if user cancelled
        if cancel_check.load(Ordering::SeqCst) {
            tracing::info!(
                "check_and_apply_all: cancelado por el usuario tras procesar {} contenedores",
                checked
            );
            // Send cancellation progress event
            update_progress(
                progress_cache,
                state_notify,
                total,
                checked,
                updated,
                num_errors,
                "__batch__".into(),
            )
            .await;
            break;
        }
    }

    // Update timestamps
    {
        let conn = db_pool.get().await.unwrap();
        let mut s = settings.lock().await;
        let cron = s
            .update_check_cron
            .clone()
            .unwrap_or_else(|| "0 0 * * *".into());
        let last_check = crate::timezone::now_formatted();
        let next_check = crate::timezone::next_cron_time(&cron).unwrap_or_default();
        for c in &containers {
            let _ = db::update_container_check_times(
                &conn.lock().unwrap(),
                &c.name,
                &last_check,
                &next_check,
            );
        }
        s.update_check_last_run_at = Some(last_check);
        let _ = db::save_settings(&conn.lock().unwrap(), &s);
    }

    // Send StateEvent if any success
    if any_success {
        tracing::info!("check_and_apply_all: enviando StateEvent para refrescar frontend");
        let containers = fetch_containers(docker, &None, db_pool).await;
        let _ = tx.send(StateEvent { containers });
    }

    tracing::info!(
        "[check_and_apply_all] FINAL: checked={} total={} updated={} errors={} any_success={}",
        checked,
        total,
        updated,
        num_errors,
        any_success
    );

    // Send final batch-complete progress event
    update_progress(
        progress_cache,
        state_notify,
        total,
        total,
        updated,
        num_errors,
        "__batch__".into(),
    )
    .await;

    // Reset progress cache after batch completes so the next state poll
    // returns default (idle) progress rather than stale "__batch__" data.
    reset_progress_cache(progress_cache, state_notify).await;

    containers
}

#[allow(clippy::too_many_arguments)]
pub async fn check_all_h(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(progress_cache): State<Arc<Mutex<BatchProgress>>>,
    State(cancel_check): State<Arc<AtomicBool>>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(update_policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    State(state_notify): State<Arc<Notify>>,
) -> Json<Vec<ContainerInfo>> {
    // Reset cancel flag at start
    cancel_check.store(false, Ordering::SeqCst);
    let containers = check_and_apply_all(
        &docker,
        &db_pool,
        &tx,
        &state_notify,
        &progress_cache,
        &cancel_check,
        &settings,
        &update_history,
        &update_policies,
    )
    .await;
    Json(containers)
}

/// Cancel a running batch check/update operation.
pub async fn cancel_check_all_h(State(cancel_check): State<Arc<AtomicBool>>) -> Json<&'static str> {
    cancel_check.store(true, Ordering::SeqCst);
    tracing::info!("cancel_check_all_h: batch cancelado por el usuario");
    Json("cancelled")
}

/// Apply a single container's update policy inline.
/// Contains the same logic as the per-container loop body in `apply_policies_background`.
#[allow(clippy::too_many_arguments)]
async fn apply_single_policy(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    state_notify: &Arc<Notify>,
    progress_cache: &Arc<Mutex<BatchProgress>>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: &DbPool,
    _tx: &broadcast::Sender<StateEvent>,
    p: &PendingUpdate,
    policy: &UpdatePolicy,
    any_success: &mut bool,
    total: u32,
    checked: u32,
    updated: u32,
    errors: u32,
) {
    if policy.action == UpdateAction::None {
        tracing::warn!(
            "apply_single_policy: política None para '{}', saltando",
            p.name
        );
        let _ = update_progress(
            progress_cache,
            state_notify,
            total,
            checked,
            updated,
            errors,
            p.name.clone(),
        )
        .await;
        return;
    }

    tracing::info!(
        "apply_single_policy: procesando '{}' con acción {:?} (imagen: {}, cleanup_old_image: {})",
        p.name,
        policy.action,
        p.image_full,
        policy.cleanup_old_image
    );

    let _ = update_progress(
        progress_cache,
        state_notify,
        total,
        checked,
        updated,
        errors,
        p.name.clone(),
    )
    .await;

    let start_time = std::time::Instant::now();
    let mut success = false;
    let pull_timeout = {
        let s = settings.lock().await;
        s.pull_timeout_secs.unwrap_or(600)
    };

    match policy.action {
        UpdateAction::Pull => {
            tracing::info!(
                "apply_single_policy: Pull '{}' desde '{}'",
                p.name,
                p.image_full
            );
            if should_skip_pull(
                &policy.action,
                p.remote_digest.as_deref(),
                p.last_pulled_digest.as_deref(),
            ) {
                tracing::info!(
                    "apply_single_policy: Pull '{}' omitido, imagen ya en caché ({})",
                    p.name,
                    p.remote_digest.as_deref().unwrap_or("")
                );
                update_progress(
                    progress_cache,
                    state_notify,
                    total,
                    checked,
                    updated,
                    errors,
                    p.name.clone(),
                )
                .await;
                // D6: el skip es un no-op real. Salir antes del bloque
                // `if success` para no notificar, ni registrar historial,
                // ni ejecutar prune. La función devuelve `()` y no hay
                // limpieza obligatoria para este caso.
                return;
            } else if pull_image(
                docker,
                &p.image_full,
                p.manifest_digest.as_deref(),
                pull_timeout,
            )
            .await
            {
                tracing::info!("apply_single_policy: Pull OK '{}'", p.name);
                persist_last_pulled_digest(db_pool, &p.name, p.remote_digest.as_deref()).await;
                update_progress(
                    progress_cache,
                    state_notify,
                    total,
                    checked,
                    updated,
                    errors,
                    p.name.clone(),
                )
                .await;
                success = true;
            } else {
                tracing::error!("apply_single_policy: Pull FALLÓ '{}'", p.name);
                let entry = UpdateHistoryEntry {
                    container: p.name.clone(),
                    image: p.image_full.clone(),
                    old_digest: p.image_id.clone(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "apply-policy-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            }
        }
        UpdateAction::PullRestart => {
            tracing::info!(
                "apply_single_policy: PullRestart '{}' desde '{}'",
                p.name,
                p.image_full
            );
            let backup = if policy.rollback_on_failure {
                tag_backup_image(docker, &p.image_full).await
            } else {
                None
            };
            if pull_image(
                docker,
                &p.image_full,
                p.manifest_digest.as_deref(),
                pull_timeout,
            )
            .await
            {
                tracing::info!(
                    "apply_single_policy: Pull OK, reiniciando contenedor '{}' (cid: {})",
                    p.name,
                    p.cid
                );
                update_progress(
                    progress_cache,
                    state_notify,
                    total,
                    checked,
                    updated,
                    errors,
                    p.name.clone(),
                )
                .await;
                match recreate_container(
                    docker,
                    &p.name,
                    &p.cid,
                    &p.image_full,
                    p.manifest_digest.as_deref(),
                )
                .await
                {
                    Ok(_) => {
                        tracing::info!(
                            "apply_single_policy: contenedor '{}' recreado correctamente",
                            p.name
                        );
                        if policy.rollback_on_failure
                            && !verify_container_healthy(docker, &p.name).await
                        {
                            tracing::warn!("apply_single_policy: rollback '{}'", p.name);
                            if let Some((backup_full, base, orig_tag)) = backup {
                                rollback_container(
                                    docker,
                                    &p.cid,
                                    &base,
                                    &orig_tag,
                                    &backup_full,
                                    &p.image_full,
                                )
                                .await;
                            }
                            update_progress(
                                progress_cache,
                                state_notify,
                                total,
                                checked,
                                updated,
                                errors,
                                p.name.clone(),
                            )
                            .await;
                        } else {
                            update_progress(
                                progress_cache,
                                state_notify,
                                total,
                                checked,
                                updated,
                                errors,
                                p.name.clone(),
                            )
                            .await;
                            success = true;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "apply_single_policy: error al reiniciar '{}': {}",
                            p.name,
                            e
                        );
                        update_progress(
                            progress_cache,
                            state_notify,
                            total,
                            checked,
                            updated,
                            errors,
                            p.name.clone(),
                        )
                        .await;
                        let entry = UpdateHistoryEntry {
                            container: p.name.clone(),
                            image: p.image_full.clone(),
                            old_digest: p.image_id.clone(),
                            new_digest: String::new(),
                            timestamp: crate::timezone::now_formatted(),
                            status: "apply-policy-restart-error".into(),
                            duration_ms: start_time.elapsed().as_millis() as u64,
                        };
                        let mut hist = update_history.lock().await;
                        hist.push(entry);
                        let conn = db_pool.get().await.unwrap();
                        let _ =
                            db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
                    }
                }
            } else {
                tracing::error!("apply_single_policy: Pull FALLÓ '{}'", p.name);
                let entry = UpdateHistoryEntry {
                    container: p.name.clone(),
                    image: p.image_full.clone(),
                    old_digest: p.image_id.clone(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "apply-policy-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            }
        }
        UpdateAction::PullRestartStack => {
            // API-only path (no `docker compose` CLI): pull + recreate the
            // service container via Bollard, same semantics as the scheduler.
            match p.compose_project {
                Some(ref project) => tracing::info!(
                    "apply_single_policy: PullRestartStack '{}' (proyecto: {})",
                    p.name,
                    project
                ),
                None => tracing::info!(
                    "apply_single_policy: PullRestartStack '{}' (sin proyecto)",
                    p.name
                ),
            }
            match pull_and_recreate(
                docker,
                &p.name,
                &p.cid,
                &p.image_full,
                p.manifest_digest.as_deref(),
                pull_timeout,
            )
            .await
            {
                Ok(()) => {
                    tracing::info!(
                        "apply_single_policy: PullRestartStack '{}' recreado vía API",
                        p.name
                    );
                    update_progress(
                        progress_cache,
                        state_notify,
                        total,
                        checked,
                        updated,
                        errors,
                        p.name.clone(),
                    )
                    .await;
                    success = true;
                }
                Err(e) => {
                    tracing::error!(
                        "apply_single_policy: PullRestartStack '{}' falló: {}",
                        p.name,
                        e
                    );
                    update_progress(
                        progress_cache,
                        state_notify,
                        total,
                        checked,
                        updated,
                        errors,
                        p.name.clone(),
                    )
                    .await;
                }
            }
        }
        _ => {
            update_progress(
                progress_cache,
                state_notify,
                total,
                checked,
                updated,
                errors,
                p.name.clone(),
            )
            .await;
        }
    }

    if success {
        tracing::info!("apply_single_policy: éxito '{}'", p.name);
        notify_all(settings, &p.name, "✅ actualizado y reiniciado").await;

        // Recreate actions also performed a successful pull, so record the
        // resulting cache digest (the `Pull` branch persists it inline).
        if should_advance_digest(&policy.action) {
            persist_last_pulled_digest(db_pool, &p.name, p.remote_digest.as_deref()).await;
        }

        // Digest of the image now in use (for history and, when the container
        // was recreated, for persisting `last_remote_digest`).
        // For stacks the recreate is API-driven, so prefer the locally
        // inspected config digest of the redeployed service.
        let new_digest = if policy.action == UpdateAction::PullRestartStack {
            let local = resolve_container_image_digest(docker, &p.name).await;
            if local.is_empty() {
                p.remote_digest.clone().unwrap_or_default()
            } else {
                local
            }
        } else if let Some(d) = &p.remote_digest {
            d.clone()
        } else {
            check_remote_digest_on_image(&p.image_full, docker).await
        };

        // `last_remote_digest` represents the image the container is actually
        // running, so it advances ONLY when the container was recreated.
        // A Pull-only policy updates the local image cache but leaves the
        // running container untouched → it MUST NOT clear `has_update` nor
        // advance the digest.
        let recreated = should_advance_digest(&policy.action);
        if let Ok(conn) = db_pool.get().await {
            if let Ok(guard) = conn.lock() {
                if recreated {
                    if let Err(e) = db::update_container_has_update(&guard, &p.name, false) {
                        tracing::error!(
                            "apply_single_policy: error clearing has_update for '{}': {}",
                            p.name,
                            e
                        );
                    }
                }
                if recreated && !new_digest.is_empty() {
                    if let Err(e) =
                        db::update_container_last_remote_digest(&guard, &p.name, &new_digest)
                    {
                        tracing::error!(
                            "apply_single_policy: error storing last_remote_digest for '{}': {}",
                            p.name,
                            e
                        );
                    }
                } else if recreated {
                    tracing::warn!("apply_single_policy: no se pudo obtener digest remoto para '{}', last_remote_digest no actualizado", p.name);
                }
            } else {
                tracing::error!(
                    "apply_single_policy: mutex de DB poisoned para '{}'",
                    p.name
                );
            }
        } else {
            tracing::error!(
                "apply_single_policy: no se pudo obtener conexión DB para '{}'",
                p.name
            );
        }
        let entry = UpdateHistoryEntry {
            container: p.name.clone(),
            image: p.image_full.clone(),
            old_digest: p.image_id.clone(),
            new_digest: new_digest.clone(),
            timestamp: crate::timezone::now_formatted(),
            status: "success".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
        };
        let mut hist = update_history.lock().await;
        hist.push(entry);
        if let Ok(conn) = db_pool.get().await {
            if let Ok(guard) = conn.lock() {
                let _ = db::append_update_history(&guard, hist.last().unwrap());
            } else {
                tracing::error!(
                    "apply_single_policy: mutex de DB poisoned al persistir historial para '{}'",
                    p.name
                );
            }
        }

        *any_success = true;

        // Limpiar imágenes dangling si la política lo indica
        if policy.cleanup_old_image {
            tracing::info!(
                "apply_single_policy: policy.cleanup_old_image=true, haciendo prune de imágenes dangling para '{}'",
                p.name
            );
            let result = prune_dangling_images(docker).await;
            log_prune_result("policy.cleanup_old_image", &result);
        }
    } else {
        tracing::warn!("apply_single_policy: fallo/no-hubo-éxito '{}'", p.name);
    }
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
async fn apply_policies_background(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    state_notify: &Arc<Notify>,
    tx: &broadcast::Sender<StateEvent>,
    progress_cache: &Arc<Mutex<BatchProgress>>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
    db_pool: &DbPool,
    pending: &[PendingUpdate],
) {
    tracing::info!(
        "apply_policies_background: iniciando con {} pendientes: {:?}",
        pending.len(),
        pending.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
    let policies = update_policies.lock().await.clone();
    let policy_map: HashMap<String, UpdatePolicy> = policies
        .into_iter()
        .map(|p| (p.container.clone(), p))
        .collect();
    let (default_action, default_cleanup, default_rollback) = {
        let s = settings.lock().await;
        (
            s.default_update_action
                .clone()
                .unwrap_or_else(|| "pull-restart".into()),
            s.default_cleanup_old_image.unwrap_or(false),
            s.default_rollback_on_failure.unwrap_or(false),
        )
    };

    let mut any_success = false;

    for p in pending {
        let policy = match policy_map.get(&p.name) {
            Some(pol) => pol.clone(),
            None => UpdatePolicy {
                container: p.name.clone(),
                action: default_action.parse().unwrap_or(UpdateAction::PullRestart),
                cleanup_old_image: default_cleanup,
                rollback_on_failure: default_rollback,
                notify_events: false,
            },
        };
        apply_single_policy(
            docker,
            settings,
            state_notify,
            progress_cache,
            update_history,
            db_pool,
            tx,
            p,
            &policy,
            &mut any_success,
            0, // total
            0, // checked
            0, // updated
            0, // errors
        )
        .await;
    }

    // Safety net: limpiar dangling images que hayan podido quedar
    tracing::info!("apply_policies_background: completado, iniciando prune de imágenes dangling");
    let result = prune_dangling_images(docker).await;
    log_prune_result("safety-net", &result);

    // Enviar StateEvent para que el frontend refresque inmediatamente
    if any_success {
        tracing::info!("apply_policies_background: enviando StateEvent para refrescar frontend");
        let containers = fetch_containers(docker, &None, db_pool).await;
        let _ = tx.send(StateEvent { containers });
    }
}

/// Prune dangling images (no tag, no container reference)
pub(crate) async fn prune_dangling_images(
    docker: &Docker,
) -> Result<ImagePruneResponse, bollard::errors::Error> {
    let mut filters = HashMap::new();
    filters.insert("dangling", vec!["true"]);
    let opts = PruneImagesOptions { filters };
    docker.prune_images(Some(opts)).await
}

/// Log the result of a prune operation
pub(crate) fn log_prune_result(
    context: &str,
    result: &Result<ImagePruneResponse, bollard::errors::Error>,
) {
    match result {
        Ok(resp) => {
            let deleted_count = resp.images_deleted.as_ref().map(|v| v.len()).unwrap_or(0);
            let reclaimed = resp.space_reclaimed.unwrap_or(0);
            if deleted_count > 0 {
                tracing::info!(
                    "prune_images [{}]: {} imágenes eliminadas, {} bytes liberados",
                    context,
                    deleted_count,
                    reclaimed
                );
            } else {
                tracing::info!(
                    "prune_images [{}]: no hay imágenes dangling para eliminar",
                    context
                );
            }
        }
        Err(e) => {
            tracing::warn!("prune_images [{}]: error: {}", context, e);
        }
    }
}

/// Verify a container is running after a restart, with progressive backoff.
/// Sleeps 3s, 6s, then 12s between checks. Returns true as soon as the
/// container is running, false after all three attempts fail.
pub(crate) async fn verify_container_healthy(docker: &Docker, name: &str) -> bool {
    let delays = [3u64, 6, 12];
    for delay in &delays {
        tokio::time::sleep(std::time::Duration::from_secs(*delay)).await;
        match find_container_by_name(docker, name).await {
            Ok(c) if c.state.as_deref() == Some("running") => return true,
            Ok(c) => {
                tracing::debug!(
                    "verify_container_healthy [{}]: state={:?}, retrying in {}s",
                    name,
                    c.state,
                    delay
                );
            }
            Err(e) => {
                tracing::warn!(
                    "verify_container_healthy [{}]: find error={}, retrying in {}s",
                    name,
                    e,
                    delay
                );
            }
        }
    }
    tracing::warn!(
        "verify_container_healthy [{}]: no healthy after 3 attempts (21s total)",
        name
    );
    false
}

/// Tag current image as backup for rollback: image:tag → image:rollback-{ts}
pub(crate) async fn tag_backup_image(
    docker: &Docker,
    image: &str,
) -> Option<(String, String, String)> {
    let ts = crate::timezone::now().format("%Y%m%d%H%M%S").to_string();
    if let Some((base, original_tag)) = image.rsplit_once(':') {
        let backup_full = format!("{}:rollback-{}", base, ts);
        let opts = TagImageOptions {
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
pub(crate) async fn rollback_container(
    docker: &Docker,
    cid: &str,
    base: &str,
    original_tag: &str,
    backup_full: &str,
    new_image: &str,
) {
    tracing::warn!("Rollback: restoring backup for {}", new_image);
    let restore_opts = TagImageOptions {
        repo: base.to_string(),
        tag: original_tag.to_string(),
    };
    let _ = docker.tag_image(backup_full, Some(restore_opts)).await;
    let _ = docker
        .restart_container(cid, None::<RestartContainerOptions>)
        .await;
    let _ = docker
        .remove_image(new_image, None::<RemoveImageOptions>, None)
        .await;
}

/// Llama a `check_remote_digest_with_docker` con la referencia de imagen completa.
/// Retorna el digest remoto o cadena vacía si falla la consulta.
async fn check_remote_digest_on_image(image_full: &str, docker: &Docker) -> String {
    match crate::updates::digest::check_remote_digest_with_docker(image_full, docker).await {
        Ok((_manifest, config, _)) => config,
        Err(_) => String::new(),
    }
}

/// Inspect a container and return the ID of the image it is running (the
/// local config digest). Returns an empty string if the container cannot be
/// inspected.
async fn resolve_container_image_digest(docker: &Docker, name: &str) -> String {
    match docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
    {
        Ok(resp) => resp.image.unwrap_or_default(),
        Err(e) => {
            tracing::warn!(
                "resolve_container_image_digest [{}]: inspect failed: {}",
                name,
                e
            );
            String::new()
        }
    }
}

// ── Test helpers (implemented in GREEN phase) ──────────────

/// Reset the progress cache to default and notify waiting state handlers.
/// This MUST be called after `check_and_apply_all` completes to clear
/// the "checking" field and prevent stale batch progress on the frontend.
pub(crate) async fn reset_progress_cache(
    progress_cache: &Arc<Mutex<BatchProgress>>,
    state_notify: &Arc<Notify>,
) {
    let mut cache = progress_cache.lock().await;
    *cache = BatchProgress::default();
    drop(cache);
    state_notify.notify_waiters();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{Mutex, Notify};

    /// Verifies that `reset_progress_cache` resets a populated progress cache to default.
    ///
    /// **Bug**: `check_and_apply_all` sends a final `update_progress` with total/checked/updated/errors
    /// values but never resets the `progress_cache` to `BatchProgress::default()`. This leaves stale
    /// "checking: __batch__" on the frontend until the next batch cycle.
    ///
    /// **Expected behavior**: After `check_and_apply_all` completes, `reset_progress_cache` is called
    /// to zero out all fields and clear the "checking" marker.
    #[tokio::test]
    async fn test_reset_progress_cache_clears_non_default_values() {
        let progress_cache: Arc<Mutex<BatchProgress>> = Arc::new(Mutex::new(BatchProgress {
            total: 5,
            checked: 5,
            updated: 2,
            errors: 0,
            checking: "__batch__".into(),
        }));
        let state_notify = Arc::new(Notify::new());

        reset_progress_cache(&progress_cache, &state_notify).await;

        let cache = progress_cache.lock().await;
        assert_eq!(cache.total, 0, "total should be reset to 0");
        assert_eq!(cache.checked, 0, "checked should be reset to 0");
        assert_eq!(cache.updated, 0, "updated should be reset to 0");
        assert_eq!(cache.errors, 0, "errors should be reset to 0");
        assert!(cache.checking.is_empty(), "checking should be cleared");
    }

    // ── recreate_image_ref ───────────────────────────────────

    /// `recreate_container` MUST write a clean `repo:tag` image reference to
    /// `Config.Image`, never a `repo:tag@sha256:...` pin generated by Alloy.
    #[test]
    fn test_recreate_image_ref_strips_digest_pin() {
        assert_eq!(
            recreate_image_ref("gitea/gitea:1.27.3@sha256:87a6deadbeef"),
            "gitea/gitea:1.27.3"
        );
    }

    #[test]
    fn test_recreate_image_ref_is_idempotent() {
        assert_eq!(recreate_image_ref("nginx:alpine"), "nginx:alpine");
        assert_eq!(recreate_image_ref("nginx"), "nginx");
    }

    // ── should_seed_last_remote_digest ───────────────────────

    /// Seed only when there is no stored digest and no update pending.
    #[test]
    fn test_should_seed_only_when_missing_and_no_update() {
        assert!(should_seed_last_remote_digest(false, false));
        assert!(!should_seed_last_remote_digest(false, true));
        assert!(!should_seed_last_remote_digest(true, false));
        assert!(!should_seed_last_remote_digest(true, true));
    }

    // ── should_advance_digest (gating de `recreated`) ────────

    /// Only recreate actions advance `last_remote_digest` (and clear
    /// `has_update`); pull-only and none leave the running image untouched.
    #[test]
    fn test_should_advance_digest_only_on_recreate() {
        assert!(should_advance_digest(&UpdateAction::PullRestart));
        assert!(should_advance_digest(&UpdateAction::PullRestartStack));
        assert!(!should_advance_digest(&UpdateAction::Pull));
        assert!(!should_advance_digest(&UpdateAction::None));
    }

    // ── should_skip_pull (pull-cache-digest) ─────────────────

    /// Scenario: Imagen ya en caché — Pull + digests iguales → skip.
    #[test]
    fn test_should_skip_pull_when_remote_matches_cache() {
        assert!(should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
    }

    /// Scenario: Imagen nueva en el remoto — Pull + digests distintos → pull.
    #[test]
    fn test_should_skip_pull_false_when_remote_differs() {
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            Some("sha256:aaa")
        ));
    }

    /// Only `Pull` may skip; recreate actions always pull.
    #[test]
    fn test_should_skip_pull_only_for_pull_action() {
        assert!(!should_skip_pull(
            &UpdateAction::PullRestart,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::PullRestartStack,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::None,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
    }

    /// Both digests must be present.
    #[test]
    fn test_should_skip_pull_requires_both_digests() {
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            None,
            Some("sha256:bbb")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            None
        ));
        assert!(!should_skip_pull(&UpdateAction::Pull, None, None));
    }

    /// Empty digests must never match.
    #[test]
    fn test_should_skip_pull_false_on_empty_digests() {
        assert!(!should_skip_pull(&UpdateAction::Pull, Some(""), Some("")));
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            Some("")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some(""),
            Some("sha256:bbb")
        ));
    }

    /// Scenario: El skip es un no-op (D6). Un `Pull` cuyo digest remoto ya
    /// coincide con el cacheado NO debe marcar éxito: sin notificación, sin
    /// entrada de historial y sin prune (aunque `cleanup_old_image=true`).
    /// Solo se registra log y se actualiza el progreso.
    #[tokio::test]
    async fn test_pull_skip_is_noop() {
        // HTTP transport never touches the local socket, so the test runs
        // without a Docker daemon; the skip path makes no Docker calls.
        let docker = Docker::connect_with_http_defaults().expect("docker client");
        let settings = Arc::new(Mutex::new(Settings::default()));
        let state_notify = Arc::new(Notify::new());
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let db_pool = db::test_pool();
        let (tx, _rx) = broadcast::channel::<StateEvent>(8);

        let p = PendingUpdate {
            name: "web".into(),
            image_full: "nginx:latest".into(),
            cid: "cid123".into(),
            image_id: "sha256:old".into(),
            remote_digest: Some("sha256:bbb".into()),
            manifest_digest: None,
            last_pulled_digest: Some("sha256:bbb".into()),
            compose_project: None,
        };
        let policy = UpdatePolicy {
            container: "web".into(),
            action: UpdateAction::Pull,
            cleanup_old_image: true,
            rollback_on_failure: false,
            notify_events: true,
        };
        let mut any_success = false;

        apply_single_policy(
            &docker,
            &settings,
            &state_notify,
            &progress_cache,
            &update_history,
            &db_pool,
            &tx,
            &p,
            &policy,
            &mut any_success,
            1,
            1,
            0,
            0,
        )
        .await;

        assert!(!any_success, "el skip no debe marcar `any_success`");
        assert!(
            update_history.lock().await.is_empty(),
            "el skip no debe añadir entrada al historial"
        );
    }
}
