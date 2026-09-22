use axum::{extract::State, http::HeaderValue, response::IntoResponse, routing::get, Json, Router};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;

#[allow(unused_imports)]
use crate::models::{ContainerInfo, StateEvent};
use crate::state::AppState;
use crate::workers::CachedContainers;

/// Long-polling state endpoint.
///
/// Subscribes to the broadcast channel and waits up to 30 seconds for the next
/// `StateEvent`. If an event arrives before the timeout, returns the updated
/// container list. Otherwise falls back to the cached container list.
async fn state_h(
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(cached_containers): State<CachedContainers>,
) -> impl IntoResponse {
    let mut rx = tx.subscribe();
    let containers = match timeout(Duration::from_secs(30), rx.recv()).await {
        Ok(Ok(evt)) => evt.containers,
        _ => {
            let cached = cached_containers.read().await;
            cached.clone().unwrap_or_default()
        }
    };

    let mut response = Json(containers).into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    response
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/state", get(state_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_state_returns_containers_on_event() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let cached: CachedContainers = Arc::new(RwLock::new(None));

        // Spawn a task that sends an event after a short delay
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            tx_clone
                .send(StateEvent {
                    containers: vec![ContainerInfo {
                        id: "abc".into(),
                        name: "nginx".into(),
                        image: "nginx".into(),
                        image_tag: "latest".into(),
                        size_mb: 10.0,
                        status: "running".into(),
                        state: "running".into(),
                        has_update: false,
                        compose_project: None,
                        ports: vec![],
                        traefik_url: None,
                        updating: false,
                        registry_url: String::new(),
                        last_check: None,
                        next_check: None,
                        last_remote_digest: String::new(),
                    }],
                })
                .unwrap();
        });

        let mut rx = tx.subscribe();
        let result = timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(result.is_ok(), "Should receive event within 2 seconds");
        if let Ok(Ok(evt)) = result {
            assert_eq!(evt.containers.len(), 1);
            assert_eq!(evt.containers[0].name, "nginx");
        }
    }

    #[tokio::test]
    async fn test_state_returns_cached_on_timeout() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let containers = vec![ContainerInfo {
            id: "xyz".into(),
            name: "redis".into(),
            image: "redis".into(),
            image_tag: "7".into(),
            size_mb: 5.0,
            status: "running".into(),
            state: "running".into(),
            has_update: false,
            compose_project: None,
            ports: vec![],
            traefik_url: None,
            updating: false,
            registry_url: String::new(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
        }];
        let cached: CachedContainers = Arc::new(RwLock::new(Some(containers.clone())));

        let mut rx = tx.subscribe();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err(), "Should timeout with no events");

        let cached_read = cached.read().await;
        let result = cached_read.clone().unwrap_or_default();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "redis");
    }

    #[tokio::test]
    async fn test_state_returns_empty_when_no_cache_and_no_events() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let cached: CachedContainers = Arc::new(RwLock::new(None));

        let mut rx = tx.subscribe();
        let result = timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(result.is_err(), "Should timeout with no events");

        let cached_read = cached.read().await;
        let result = cached_read.clone().unwrap_or_default();
        assert!(result.is_empty());
    }
}
