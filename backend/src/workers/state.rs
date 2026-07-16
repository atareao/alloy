use bollard::{system::EventsOptions, Docker};
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::config::Config;
use crate::containers::fetch_containers;
use crate::models::*;

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
    config: &Config,
    settings: &Arc<Mutex<Settings>>,
    tx: &broadcast::Sender<StateEvent>,
    cache: &CachedContainers,
) {
    let monitored = settings.lock().await.monitored_containers.clone();
    let containers = fetch_containers(docker, &config.allowed_containers, &monitored).await;
    *cache.write().await = Some(containers.clone());
    let _ = tx.send(StateEvent { containers });
}

pub async fn state_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    refresh(&docker, &config, &settings, &tx, &cached_containers).await;

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
                                        refresh(&docker, &config, &settings, &tx, &cached_containers).await;
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
                    refresh(&docker, &config, &settings, &tx, &cached_containers).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
