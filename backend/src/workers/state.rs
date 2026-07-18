use bollard::{system::EventsOptions, Docker};
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::containers::fetch_containers;
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
                let image_id = c.image_id.as_deref().map(|s| s.to_string());
                Some((name, image, id, image_id))
            })
            .collect(),
        Err(_) => vec![],
    }
}

async fn refresh(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    tx: &broadcast::Sender<StateEvent>,
    cache: &CachedContainers,
    notif_tx: &broadcast::Sender<NotifEvent>,
    previous_states: &mut HashMap<String, String>,
) {
    let monitored = settings.lock().await.monitored_containers.clone();
    let containers = fetch_containers(docker, &None, &monitored).await;
    *cache.write().await = Some(containers.clone());
    let _ = tx.send(StateEvent {
        containers: containers.clone(),
    });

    // Detect state changes and send notifications
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    let settings_arc = settings.clone();
    for c in &containers {
        let prev = previous_states
            .get(&c.name)
            .map(|s| s.as_str())
            .unwrap_or("");
        let curr = c.state.as_str();
        if !prev.is_empty() && prev != curr {
            let status_msg = match curr {
                "running" => "▶️ en ejecución",
                "exited" => "⏹️ detenido",
                "paused" => "⏸️ pausado",
                "restarting" => "🔄 reiniciando",
                "dead" => "💀 finalizado",
                "created" => "🆕 creado",
                "removing" => "🗑️ eliminando",
                _ => curr,
            };
            let _ = notif_tx.send(NotifEvent {
                container: c.name.clone(),
                status: status_msg.to_string(),
                timestamp: now.clone(),
            });
            notify_all(&settings_arc, &c.name, status_msg).await;
        }
    }
    // Update previous states
    for c in &containers {
        previous_states.insert(c.name.clone(), c.state.clone());
    }
    // Remove stale entries (containers that no longer exist)
    let current_names: std::collections::HashSet<String> =
        containers.iter().map(|c| c.name.clone()).collect();
    previous_states.retain(|k, _| current_names.contains(k));
}

pub async fn state_worker(
    docker: Docker,
    settings: Arc<Mutex<Settings>>,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
    notif_tx: broadcast::Sender<NotifEvent>,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    let mut previous_states: HashMap<String, String> = HashMap::new();

    refresh(
        &docker,
        &settings,
        &tx,
        &cached_containers,
        &notif_tx,
        &mut previous_states,
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
            refresh(&docker,  &settings, &tx, &cached_containers, &notif_tx, &mut previous_states).await;
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
                                refresh(&docker,  &settings, &tx, &cached_containers, &notif_tx, &mut previous_states).await;
                            }
                        }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
