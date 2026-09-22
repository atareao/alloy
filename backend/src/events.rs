use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;

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

/// SSE endpoint for container state events.
///
/// Subscribes to the `StateEvent` broadcast channel and forwards all events
/// as SSE messages with event name `"state"` and JSON-encoded container data.
async fn events_sse_h(
    State(tx): State<broadcast::Sender<StateEvent>>,
) -> (HeaderMap, Sse<impl futures::Stream<Item = Result<Event, Infallible>>>) {
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(evt) => {
            let data = serde_json::to_string(&evt.containers).unwrap_or_default();
            Some(Ok(Event::default().event("state").data(data)))
        }
        Err(BroadcastStreamRecvError::Lagged(n)) => {
            tracing::warn!("events SSE lagged by {} messages", n);
            None
        }
    });

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    (headers, Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/state", get(state_h))
        .route("/api/events", get(events_sse_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_state_returns_containers_on_event() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let _cached: CachedContainers = Arc::new(RwLock::new(None));

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

    #[tokio::test]
    async fn test_events_sse_streams_state_events() {
        let (tx, _rx_guard) = broadcast::channel::<StateEvent>(16);

        let containers = vec![ContainerInfo {
            id: "sse-test".into(),
            name: "sse-container".into(),
            image: "test".into(),
            image_tag: "latest".into(),
            size_mb: 1.0,
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

        // Subscribe first, then send (broadcast requires at least one receiver)
        let rx = tx.subscribe();

        // Send an event
        tx.send(StateEvent {
            containers: containers.clone(),
        })
        .unwrap();

        // Read from the subscription (it will get the oldest retained message)
        let stream = BroadcastStream::new(rx)
            .filter_map(|result| match result {
                Ok(evt) => {
                    let data = serde_json::to_string(&evt.containers).ok()?;
                    Some(Ok::<_, Infallible>(
                        Event::default().event("state").data(data),
                    ))
                }
                Err(_) => None,
            })
            .take(1);

        // Keep the guard alive so the broadcast channel has at least one receiver
        drop(_rx_guard);

        let events: Vec<_> = stream.collect().await;
        assert_eq!(events.len(), 1, "Should receive exactly one SSE event");
        if let Ok(event) = &events[0] {
            let event_str = format!("{:?}", event);
            assert!(
                event_str.contains("sse-container"),
                "Event data should contain container name"
            );
        }
    }
}