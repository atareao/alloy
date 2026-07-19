use axum::{
    extract::{Path, State},
    response::Json,
};
use bollard::{
    container::{ListContainersOptions, RestartContainerOptions},
    Docker,
};
use chrono::Local;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::containers::{fetch_containers, find_container_by_name, pull_image};
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::check_remote_digest;
use crate::workers::auto_update::{rollback_container, tag_backup_image, verify_container_healthy};
use crate::workers::resolve_compose_file;

struct PendingUpdate {
    name: String,
    image_full: String,
    cid: String,
    compose_project: Option<String>,
}

pub async fn update_container_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
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
        let conn = db_pool.get().await.unwrap();
        let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
        drop(conn);
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
            notify_all(&settings, &name, "✅ actualizado y reiniciado").await;
            {
                let conn = db_pool.get().await.unwrap();
                let _ = db::update_container_has_update(&conn.lock().unwrap(), &name, false);
            }
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
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
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
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
            Err(AppError::Docker(format!("restart: {}", e)))
        }
    }
}

pub async fn update_all_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
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
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
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
                notify_all(&settings, &name, "✅ actualizado").await;
                {
                    let conn = db_pool.get().await.unwrap();
                    let _ = db::update_container_has_update(&conn.lock().unwrap(), &name, false);
                }
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
                let conn = db_pool.get().await.unwrap();
                let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
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
    State(db_pool): State<db::DbPool>,
    Path(name): Path<String>,
) -> Result<Json<VersionCompare>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image_full = container.image.as_deref().unwrap_or("");
    let (has_update, local_tag, remote_digest, remote_tag, error) = if image_full.is_empty() {
        (None, "unknown".into(), None, None, Some("no image".into()))
    } else {
        let (repo, local_tag) = crate::updates::digest::parse_image_ref(image_full);
        let (remote_digest, remote_tag, error) = match check_remote_digest(&repo, &local_tag).await
        {
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

#[allow(clippy::too_many_arguments)]
pub async fn check_all_h(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(update_policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
) -> Json<Vec<ContainerInfo>> {
    let mut containers = fetch_containers(&docker, &None, &db_pool).await;
    let raw_containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
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
    let update_map: HashMap<String, bool> = results.into_iter().collect();
    for c in &mut containers {
        c.has_update = *update_map.get(&c.name).unwrap_or(&false);
    }
    let conn = db_pool.get().await.unwrap();
    for c in &containers {
        let _ = db::update_container_has_update(&conn.lock().unwrap(), &c.name, c.has_update);
    }
    drop(conn);

    let pending: Vec<PendingUpdate> = containers
        .iter()
        .filter(|c| c.has_update)
        .filter_map(|c| {
            let raw = raw_containers.iter().find(|ct| {
                ct.names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n) == c.name.as_str())
                    .unwrap_or(false)
            })?;
            let image_full = raw.image.as_deref()?.to_string();
            let cid = raw.id.as_deref()?.to_string();
            let compose_project = raw
                .labels
                .as_ref()
                .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                .cloned();
            Some(PendingUpdate {
                name: c.name.clone(),
                image_full,
                cid,
                compose_project,
            })
        })
        .collect();

    if !pending.is_empty() {
        let docker = docker.clone();

        let settings = settings.clone();
        let update_tx = update_tx.clone();
        let notif_tx = notif_tx.clone();
        let update_history = update_history.clone();
        let update_policies = update_policies.clone();
        tokio::spawn(async move {
            apply_policies_background(
                &docker,
                &settings,
                &update_tx,
                &notif_tx,
                &update_history,
                &update_policies,
                &db_pool,
                &pending,
            )
            .await;
        });
    }

    Json(containers)
}

#[allow(clippy::too_many_arguments)]
async fn apply_policies_background(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    update_tx: &broadcast::Sender<UpdateProgress>,
    notif_tx: &broadcast::Sender<NotifEvent>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
    db_pool: &DbPool,
    pending: &[PendingUpdate],
) {
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
        if policy.action == UpdateAction::None {
            continue;
        }

        let _ = update_tx.send(UpdateProgress {
            container: p.name.clone(),
            status: format!("🔄 actualizando {}...", p.name),
            done: false,
            error: None,
        });

        let start_time = std::time::Instant::now();
        let mut success = false;

        match policy.action {
            UpdateAction::Pull => {
                if pull_image(docker, &p.image_full).await {
                    let _ = update_tx.send(UpdateProgress {
                        container: p.name.clone(),
                        status: "✅ pulled".into(),
                        done: true,
                        error: None,
                    });
                    success = true;
                }
            }
            UpdateAction::PullRestart => {
                if policy.cleanup_old_image || policy.rollback_on_failure {
                    let backup = if policy.rollback_on_failure {
                        tag_backup_image(docker, &p.image_full).await
                    } else {
                        None
                    };
                    if pull_image(docker, &p.image_full).await {
                        let _ = docker
                            .restart_container(&p.cid, None::<RestartContainerOptions>)
                            .await;
                        if policy.rollback_on_failure
                            && !verify_container_healthy(docker, &p.name).await
                        {
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
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "⚠️ rollback aplicado".into(),
                                done: true,
                                error: Some("container no healthy".into()),
                            });
                        } else {
                            if policy.cleanup_old_image {
                                let _ = docker
                                    .remove_image(
                                        &p.cid,
                                        None::<bollard::image::RemoveImageOptions>,
                                        None,
                                    )
                                    .await;
                            }
                            let _ = update_tx.send(UpdateProgress {
                                container: p.name.clone(),
                                status: "✅ actualizado + reiniciado".into(),
                                done: true,
                                error: None,
                            });
                            success = true;
                        }
                    }
                } else {
                    if pull_image(docker, &p.image_full).await {
                        let _ = docker
                            .restart_container(&p.cid, None::<RestartContainerOptions>)
                            .await;
                        let _ = update_tx.send(UpdateProgress {
                            container: p.name.clone(),
                            status: "✅ actualizado + reiniciado".into(),
                            done: true,
                            error: None,
                        });
                        success = true;
                    }
                }
            }
            UpdateAction::PullRestartStack => {
                if let Some(ref project) = p.compose_project {
                    let compose_file = resolve_compose_file(docker, project).await;
                    if let Some(ref file) = compose_file {
                        let _ = update_tx.send(UpdateProgress {
                            container: p.name.clone(),
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
                                    container: p.name.clone(),
                                    status: "✅ stack updated".into(),
                                    done: true,
                                    error: None,
                                });
                                success = true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        if success {
            let _ = notif_tx.send(NotifEvent {
                container: p.name.clone(),
                status: "updated ✅".into(),
                timestamp: Local::now().format("%H:%M:%S").to_string(),
            });
            notify_all(settings, &p.name, "✅ actualizado y reiniciado").await;
            {
                let conn = db_pool.get().await.unwrap();
                let _ = db::update_container_has_update(&conn.lock().unwrap(), &p.name, false);
            }
            let entry = UpdateHistoryEntry {
                container: p.name.clone(),
                image: p.image_full.clone(),
                old_digest: String::new(),
                new_digest: String::new(),
                timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                status: "success".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
            };
            let mut hist = update_history.lock().await;
            hist.push(entry);
            let conn = db_pool.get().await.unwrap();
            let _ = db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
        }
    }
}
