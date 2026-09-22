use axum::{extract::State, http::HeaderValue, response::IntoResponse, routing::get, Json, Router};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio::time::timeout;

use crate::models::*;
use crate::state::AppState;
use crate::workers::CachedContainers;

/// Long-polling state endpoint using tokio::sync::Notify.
///
/// If cached containers are populated, returns immediately.
/// Otherwise, waits up to 30 seconds for a notification (from state worker or update
/// operations), then returns the current cached containers + summary + progress.
async fn state_h(
    State(cached_containers): State<CachedContainers>,
    State(progress_cache): State<Arc<Mutex<BatchProgress>>>,
    State(state_notify): State<Arc<Notify>>,
) -> impl IntoResponse {
    // If cache is populated, return immediately
    let should_wait = {
        let cached = cached_containers.read().await;
        cached.is_none()
    };

    if should_wait {
        // Cache empty, wait for first notification
        let _ = timeout(Duration::from_secs(30), state_notify.notified()).await;
    }

    // Read current state
    let containers = cached_containers.read().await.clone().unwrap_or_default();
    let progress = progress_cache.lock().await.clone();
    let summary = compute_summary(&containers);

    let mut response = Json(StateResponse {
        containers,
        summary,
        progress,
    })
    .into_response();
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
    use crate::models::{ContainerInfo, StateEvent};
    use std::sync::Arc;
    use tokio::sync::{broadcast, Mutex, Notify, RwLock};

    // ── helpers ─────────────────────────────────────────────

    fn make_container(name: &str, state: &str, has_update: bool) -> ContainerInfo {
        ContainerInfo {
            id: name.to_string(),
            name: name.to_string(),
            image: format!("{}:latest", name),
            image_tag: "latest".into(),
            size_mb: 10.0,
            status: state.to_string(),
            state: state.to_string(),
            has_update,
            compose_project: None,
            ports: vec![],
            traefik_url: None,
            updating: false,
            registry_url: String::new(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
        }
    }

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

    // ── Nuevos tests: state_h unificado con Notify + StateResponse ──

    #[tokio::test]
    async fn test_state_notify_returns_state_response_on_notify() {
        use crate::models::{compute_summary, StateResponse};

        let containers = vec![
            make_container("web", "running", false),
            make_container("api", "running", true),
        ];
        let cached: CachedContainers = Arc::new(RwLock::new(Some(containers.clone())));
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let state_notify = Arc::new(Notify::new());

        // Spawn a task that notifies after a short delay (simulating state worker)
        let n2 = state_notify.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            n2.notify_one();
        });

        // Simulate handler: wait on notify with timeout
        let result = timeout(Duration::from_secs(2), state_notify.notified()).await;
        assert!(result.is_ok(), "Should be notified within 2 seconds");

        // Re-read cache and progress, build StateResponse (as handler does)
        let cached_read = cached.read().await;
        let refreshed = cached_read.clone().unwrap_or_default();
        let progress = progress_cache.lock().await;

        let summary = compute_summary(&refreshed);
        let response = StateResponse {
            containers: refreshed,
            summary,
            progress: progress.clone(),
        };

        assert_eq!(response.containers.len(), 2);
        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.running, 2);
        assert_eq!(response.summary.with_updates, 1);
        assert_eq!(response.progress.total, 0);
    }

    #[tokio::test]
    async fn test_state_notify_returns_cached_on_timeout() {
        use crate::models::{compute_summary, StateResponse};

        let containers = vec![
            make_container("redis", "running", false),
            make_container("postgres", "exited", false),
        ];
        let cached: CachedContainers = Arc::new(RwLock::new(Some(containers.clone())));
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let state_notify = Arc::new(Notify::new());

        // Simulate handler: wait on notify with short timeout (no notification will arrive)
        let result = timeout(Duration::from_millis(50), state_notify.notified()).await;
        assert!(result.is_err(), "Should timeout with no notify");

        // Fallback to cache (as handler does after timeout)
        let cached_read = cached.read().await;
        let refreshed = cached_read.clone().unwrap_or_default();
        let progress = progress_cache.lock().await;

        let summary = compute_summary(&refreshed);
        let response = StateResponse {
            containers: refreshed,
            summary,
            progress: progress.clone(),
        };

        assert_eq!(response.containers.len(), 2);
        assert_eq!(response.summary.total, 2);
        assert_eq!(response.summary.running, 1);
        assert_eq!(response.summary.stopped, 1);
        assert_eq!(response.progress.total, 0);
    }

    #[tokio::test]
    async fn test_state_notify_returns_empty_when_no_cache() {
        use crate::models::{compute_summary, StateResponse};

        // cached_containers is None (not yet populated)
        let cached: CachedContainers = Arc::new(RwLock::new(None));
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let state_notify = Arc::new(Notify::new());

        // Simulate handler: wait on notify with short timeout
        let result = timeout(Duration::from_millis(50), state_notify.notified()).await;
        assert!(result.is_err(), "Should timeout with no notify");

        // Fallback to cache → None → empty vec
        let cached_read = cached.read().await;
        let refreshed = cached_read.clone().unwrap_or_default();
        let progress = progress_cache.lock().await;

        let summary = compute_summary(&refreshed);
        let response = StateResponse {
            containers: refreshed,
            summary,
            progress: progress.clone(),
        };

        assert!(response.containers.is_empty());
        assert_eq!(response.summary.total, 0);
        assert_eq!(response.summary.running, 0);
        assert_eq!(response.summary.stopped, 0);
        assert_eq!(response.summary.paused, 0);
        assert_eq!(response.summary.with_updates, 0);
        assert_eq!(response.progress.total, 0);
    }
}
