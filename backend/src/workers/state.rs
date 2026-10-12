use bollard::{container::InspectContainerOptions, system::EventsOptions, Docker};
use futures::{pin_mut, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, Notify, RwLock};

use crate::containers::fetch_containers;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;

pub type CachedContainers = Arc<RwLock<Option<Vec<ContainerInfo>>>>;

pub async fn docker_list_running(docker: &Docker) -> Vec<(String, String, String, Option<String>)> {
    match docker
        .list_containers(Some(bollard::container::ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await
    {
        Ok(list) => {
            // Pre-resolve: bare digest images (sha256:...) only carry the digest
            // in `image`; inspect each container to recover the real image ref.
            let mut resolved_images: HashMap<String, String> = HashMap::new();
            for c in &list {
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
            list.iter()
                .filter_map(|c| {
                    let name = c
                        .names
                        .as_ref()
                        .and_then(|n| n.first())
                        .map(|n| strip_name(n))?;
                    let raw_image = c.image.as_deref()?;
                    let image = if raw_image.starts_with("sha256:") {
                        c.id.as_ref()
                            .and_then(|cid| resolved_images.get(cid))
                            .map(|s| s.as_str())
                            .unwrap_or(raw_image)
                            .to_string()
                    } else {
                        raw_image.to_string()
                    };
                    let id = c.id.as_deref()?.to_string();
                    let image_id = c.image_id.as_deref().map(|s| s.to_string());
                    Some((name, image, id, image_id))
                })
                .collect()
        }
        Err(_) => vec![],
    }
}

/// Normalized Docker action inferred from a container state transition.
fn infer_action(prev: &str, curr: &str) -> &'static str {
    match curr {
        "running" if prev == "paused" => "unpause",
        "running" => "start",
        "paused" => "pause",
        "restarting" => "restart",
        "created" => "create",
        "removing" => "destroy",
        "dead" => "die",
        "exited" => "stop",
        _ => "update",
    }
}

/// Human-readable status message for a container state-change notification.
fn status_message(state: &str) -> String {
    match state {
        "running" => "▶️ en ejecución",
        "exited" => "⏹️ detenido",
        "paused" => "⏸️ pausado",
        "restarting" => "🔄 reiniciando",
        "dead" => "💀 finalizado",
        "created" => "🆕 creado",
        "removing" => "🗑️ eliminando",
        other => other,
    }
    .to_string()
}

/// Pure helper: build one [`ContainerEvent`] for every container whose state
/// changed since the previous cycle, and none for unchanged containers.
pub fn detect_state_changes(
    previous_states: &HashMap<String, String>,
    containers: &[ContainerInfo],
) -> Vec<ContainerEvent> {
    let timestamp = crate::timezone::now_formatted();
    containers
        .iter()
        .filter_map(|c| {
            let prev = previous_states.get(&c.name)?;
            if prev.is_empty() || prev == &c.state {
                return None;
            }
            Some(ContainerEvent {
                name: c.name.clone(),
                action: infer_action(prev, &c.state).to_string(),
                state: c.state.clone(),
                timestamp: timestamp.clone(),
            })
        })
        .collect()
}

/// Pure helper: notifications to dispatch for state changes, honoring the
/// per-container `notify_events` policy and skipping containers currently being
/// updated by the scheduler. Returns `(container, status_message)` pairs.
pub fn detect_notifications(
    previous_states: &HashMap<String, String>,
    containers: &[ContainerInfo],
    policies: &[UpdatePolicy],
    update_in_progress: &HashSet<String>,
) -> Vec<(String, String)> {
    containers
        .iter()
        .filter_map(|c| {
            let prev = previous_states.get(&c.name)?;
            if prev.is_empty() || prev == &c.state {
                return None;
            }
            if update_in_progress.contains(&c.name) {
                return None;
            }
            let should_notify = policies
                .iter()
                .find(|p| p.container == c.name)
                .map(|p| p.notify_events)
                .unwrap_or(false);
            if !should_notify {
                return None;
            }
            Some((c.name.clone(), status_message(&c.state)))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
async fn refresh(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
    tx: &broadcast::Sender<StateEvent>,
    cache: &CachedContainers,
    previous_states: &mut HashMap<String, String>,
    db_pool: &DbPool,
    update_in_progress: &Arc<Mutex<HashSet<String>>>,
    state_notify: &Arc<Notify>,
    notif_tx: &broadcast::Sender<NotifEvent>,
    container_event_tx: &broadcast::Sender<ContainerEvent>,
) {
    let containers = fetch_containers(docker, &None, db_pool).await;
    *cache.write().await = Some(containers.clone());
    let _ = tx.send(StateEvent {
        containers: containers.clone(),
    });

    // Incremental highlights: one `container-event` per changed container.
    for evt in detect_state_changes(previous_states, &containers) {
        let _ = container_event_tx.send(evt);
    }

    // UI notifications: reuse the existing `notify_events` trigger and also
    // expose it on the `notif_tx` broadcast channel for the SSE stream.
    let policies = update_policies.lock().await;
    let notifications = {
        let in_progress = update_in_progress.lock().await;
        detect_notifications(previous_states, &containers, &policies, &in_progress)
    };
    drop(policies);
    for (name, status_msg) in notifications {
        notify_all(settings, &name, &status_msg).await;
        let _ = notif_tx.send(NotifEvent {
            container: name,
            status: status_msg,
            timestamp: crate::timezone::now_formatted(),
        });
    }

    // Update previous states
    for c in &containers {
        previous_states.insert(c.name.clone(), c.state.clone());
    }
    // Remove stale entries (containers that no longer exist)
    let current_names: HashSet<String> = containers.iter().map(|c| c.name.clone()).collect();
    previous_states.retain(|k, _| current_names.contains(k));

    // Notify long-polling state endpoint that state has changed
    state_notify.notify_waiters();
}

#[allow(clippy::too_many_arguments)]
pub async fn state_worker(
    docker: Docker,
    settings: Arc<Mutex<Settings>>,
    update_policies: Arc<Mutex<Vec<UpdatePolicy>>>,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
    db_pool: DbPool,
    update_in_progress: Arc<Mutex<HashSet<String>>>,
    state_notify: Arc<Notify>,
    notif_tx: broadcast::Sender<NotifEvent>,
    container_event_tx: broadcast::Sender<ContainerEvent>,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    let mut previous_states: HashMap<String, String> = HashMap::new();

    refresh(
        &docker,
        &settings,
        &update_policies,
        &tx,
        &cached_containers,
        &mut previous_states,
        &db_pool,
        &update_in_progress,
        &state_notify,
        &notif_tx,
        &container_event_tx,
    )
    .await;

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
            refresh(&docker, &settings, &update_policies, &tx, &cached_containers, &mut previous_states, &db_pool, &update_in_progress, &state_notify, &notif_tx, &container_event_tx).await;
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
                                refresh(&docker, &settings, &update_policies, &tx, &cached_containers, &mut previous_states, &db_pool, &update_in_progress, &state_notify, &notif_tx, &container_event_tx).await;
                            }
                        }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ContainerInfo, UpdateAction, UpdatePolicy};
    use std::collections::{HashMap, HashSet};

    fn make_container(name: &str, state: &str) -> ContainerInfo {
        ContainerInfo {
            id: name.into(),
            name: name.into(),
            image: format!("{name}:latest"),
            image_tag: "latest".into(),
            size_mb: 1.0,
            status: state.into(),
            state: state.into(),
            has_update: false,
            updating: false,
            compose_project: None,
            ports: vec![],
            traefik_url: None,
            registry_url: String::new(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }
    }

    fn policy(container: &str, notify_events: bool) -> UpdatePolicy {
        UpdatePolicy {
            container: container.into(),
            action: UpdateAction::None,
            cleanup_old_image: false,
            rollback_on_failure: false,
            notify_events,
        }
    }

    // ── detect_state_changes (task 1.3) ─────────────────────

    #[test]
    fn test_detect_state_changes_only_changed_containers() {
        let mut prev = HashMap::new();
        prev.insert("nginx".to_string(), "exited".to_string());
        prev.insert("redis".to_string(), "running".to_string());

        let containers = vec![
            make_container("nginx", "running"),
            make_container("redis", "running"),
        ];

        let changes = detect_state_changes(&prev, &containers);
        assert_eq!(
            changes.len(),
            1,
            "only containers with prev != curr must emit events"
        );
        assert_eq!(changes[0].name, "nginx");
        assert_eq!(changes[0].state, "running");
        assert_eq!(changes[0].action, "start");
        assert!(!changes[0].timestamp.is_empty());
    }

    #[test]
    fn test_detect_state_changes_empty_when_unchanged() {
        let mut prev = HashMap::new();
        prev.insert("nginx".to_string(), "running".to_string());
        let containers = vec![make_container("nginx", "running")];
        assert!(
            detect_state_changes(&prev, &containers).is_empty(),
            "no event for an unchanged container"
        );
    }

    #[test]
    fn test_detect_state_changes_skips_unknown_containers() {
        let prev = HashMap::new();
        let containers = vec![make_container("brand-new", "running")];
        assert!(
            detect_state_changes(&prev, &containers).is_empty(),
            "container absent from previous cycle must not emit"
        );
    }

    #[test]
    fn test_infer_action_normalizes_transitions() {
        assert_eq!(infer_action("exited", "running"), "start");
        assert_eq!(infer_action("paused", "running"), "unpause");
        assert_eq!(infer_action("running", "paused"), "pause");
        assert_eq!(infer_action("running", "exited"), "stop");
        assert_eq!(infer_action("running", "restarting"), "restart");
        assert_eq!(infer_action("created", "running"), "start");
    }

    // ── detect_notifications (task 1.5) ─────────────────────

    #[test]
    fn test_detect_notifications_only_for_notify_events_policy() {
        let mut prev = HashMap::new();
        prev.insert("nginx".to_string(), "exited".to_string());
        prev.insert("redis".to_string(), "exited".to_string());
        let containers = vec![
            make_container("nginx", "running"),
            make_container("redis", "running"),
        ];
        let policies = vec![policy("nginx", true), policy("redis", false)];
        let in_progress = HashSet::new();

        let notifications = detect_notifications(&prev, &containers, &policies, &in_progress);
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].0, "nginx");
        assert!(!notifications[0].1.is_empty());
    }

    #[test]
    fn test_detect_notifications_skips_update_in_progress() {
        let mut prev = HashMap::new();
        prev.insert("nginx".to_string(), "exited".to_string());
        let containers = vec![make_container("nginx", "running")];
        let policies = vec![policy("nginx", true)];
        let mut in_progress = HashSet::new();
        in_progress.insert("nginx".to_string());

        assert!(
            detect_notifications(&prev, &containers, &policies, &in_progress).is_empty(),
            "containers being updated by the scheduler must not notify"
        );
    }

    #[test]
    fn test_detect_notifications_empty_when_no_change() {
        let mut prev = HashMap::new();
        prev.insert("nginx".to_string(), "running".to_string());
        let containers = vec![make_container("nginx", "running")];
        let policies = vec![policy("nginx", true)];
        let in_progress = HashSet::new();
        assert!(detect_notifications(&prev, &containers, &policies, &in_progress).is_empty());
    }
}
