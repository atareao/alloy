use bollard::{system::EventsOptions, Docker};
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

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
    tx: &broadcast::Sender<StateEvent>,
    cache: &CachedContainers,
) {
    let containers = fetch_containers(docker, &config.allowed_containers).await;
    *cache.write().await = Some(containers.clone());
    let _ = tx.send(StateEvent { containers });
}

pub async fn state_worker(
    docker: Docker,
    config: Config,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    refresh(&docker, &config, &tx, &cached_containers).await;

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
                                        refresh(&docker, &config, &tx, &cached_containers).await;
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
                    refresh(&docker, &config, &tx, &cached_containers).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::Docker;

    fn is_podman_available() -> bool {
        std::env::var("DOCKER_HOST").is_ok()
            || std::path::Path::new("/run/user/1000/podman/podman.sock").exists()
    }

    async fn podman_client() -> Docker {
        let socket = std::env::var("DOCKER_HOST")
            .unwrap_or_else(|_| "unix:///run/user/1000/podman/podman.sock".to_string());
        Docker::connect_with_local(&socket, 120, bollard::API_DEFAULT_VERSION)
            .expect("Failed to connect to Podman socket")
    }

    #[tokio::test]
    async fn test_integration_docker_list_running_returns_containers() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let running = docker_list_running(&docker).await;
        assert!(
            !running.is_empty(),
            "Should have at least one running container"
        );
    }

    #[tokio::test]
    async fn test_integration_docker_list_running_alloy_present() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let running = docker_list_running(&docker).await;
        let alloy = running.iter().find(|(name, _, _, _)| name == "alloy");
        assert!(
            alloy.is_some(),
            "Container 'alloy' should be in running list"
        );
        let (name, image, id, image_id) = alloy.unwrap();
        assert!(!image.is_empty());
        assert!(!id.is_empty());
        println!(
            "alloy: name={}, image={}, id={}, image_id={:?}",
            name, image, id, image_id
        );
    }

    #[tokio::test]
    async fn test_integration_docker_list_running_oxinbox_present() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let running = docker_list_running(&docker).await;
        let oxinbox = running.iter().find(|(name, _, _, _)| name == "oxinbox");
        assert!(
            oxinbox.is_some(),
            "Container 'oxinbox' should be in running list"
        );
    }

    #[tokio::test]
    async fn test_integration_docker_list_running_structure() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let running = docker_list_running(&docker).await;
        for (name, image, id, image_id) in &running {
            assert!(!name.is_empty(), "Name should not be empty");
            assert!(!image.is_empty(), "Image should not be empty for {}", name);
            assert!(!id.is_empty(), "ID should not be empty for {}", name);
            if let Some(img_id) = image_id {
                assert!(!img_id.is_empty(), "image_id should not be empty string");
            }
        }
    }
}
