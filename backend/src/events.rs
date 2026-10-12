use axum::{
    extract::State,
    http::HeaderValue,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::get,
    Json, Router,
};
use futures::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, Notify};
use tokio::time::timeout;
use tokio_stream::wrappers::{BroadcastStream, IntervalStream};

use crate::models::*;
use crate::state::AppState;
use crate::workers::CachedContainers;

/// Long-polling state endpoint using tokio::sync::Notify.
///
/// Always waits for a notification up to 30 seconds, regardless of cache state.
/// If `notify_waiters()` was called since the last `notified()` was polled, the
/// future completes immediately — otherwise it waits until the next notification.
/// Returns the current cached containers + summary + progress.
async fn state_h(
    State(cached_containers): State<CachedContainers>,
    State(progress_cache): State<Arc<Mutex<BatchProgress>>>,
    State(state_notify): State<Arc<Notify>>,
) -> impl IntoResponse {
    // Always wait for notification (long-poll). If a notification is pending
    // (notify_waiters() was called since last poll), notified() returns immediately.
    // Otherwise, waits up to 30 seconds.
    let _ = timeout(Duration::from_secs(30), state_notify.notified()).await;

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

/// Wait on the state Notify with a timeout, regardless of whether the cache is populated.
/// This ensures long-polling clients always get fresh data (or timeout + fallback to cache).
#[allow(dead_code)]
pub(crate) async fn state_wait_on_notify(
    _cached_containers: &CachedContainers,
    state_notify: &Arc<Notify>,
) {
    let _ = timeout(Duration::from_secs(30), state_notify.notified()).await;
}

/// Serialize `payload` as the JSON `data` of a named SSE event.
///
/// Falls back to an empty JSON object if serialization ever fails, so a single
/// malformed payload can never tear down the stream.
fn json_event<T: serde::Serialize>(name: &str, payload: &T) -> Event {
    Event::default()
        .event(name)
        .json_data(payload)
        .unwrap_or_else(|_| Event::default().event(name).data("{}"))
}

/// Build the merged SSE stream served by `GET /api/stream`.
///
/// Emits an initial `snapshot` event (a [`StateResponse`] built from
/// `cached_containers` + `compute_summary` + `progress_cache`) **before** any
/// incremental event, then chains the four merged sources:
///
/// - `StateEvent`  → `snapshot` (full state after each worker refresh)
/// - `ContainerEvent` → `container-event`
/// - `UpdateProgress` → `update-progress`
/// - `NotifEvent` → `notification`
///
/// plus a `: keepalive` comment emitted every 5 seconds. The heartbeat is
/// produced by merging a `tokio::time::interval` stream — deliberately **not**
/// via `Sse::keep_alive()` — so real events are never delayed.
pub(crate) async fn build_event_stream(
    cached_containers: CachedContainers,
    progress_cache: Arc<Mutex<BatchProgress>>,
    tx: broadcast::Sender<StateEvent>,
    progress_tx: broadcast::Sender<UpdateProgress>,
    notif_tx: broadcast::Sender<NotifEvent>,
    container_event_tx: broadcast::Sender<ContainerEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // ── Initial snapshot (delivered before any incremental event) ──
    let containers = cached_containers.read().await.clone().unwrap_or_default();
    let progress = progress_cache.lock().await.clone();
    let summary = compute_summary(&containers);
    let snapshot = StateResponse {
        containers,
        summary,
        progress,
    };
    let initial = stream::once(async move {
        Ok(Event::default()
            .event("snapshot")
            .json_data(&snapshot)
            .unwrap_or_else(|_| Event::default().event("snapshot").data("{}")))
    });

    // ── State refreshes are full snapshots (StateEvent carries the full list).
    // Enrich with summary + current progress so the payload matches StateResponse.
    let progress_for_state = progress_cache.clone();
    let state_stream = BroadcastStream::new(tx.subscribe()).filter_map(move |res| {
        let progress_cache = progress_for_state.clone();
        async move {
            let evt = res.ok()?;
            let summary = compute_summary(&evt.containers);
            let progress = progress_cache.lock().await.clone();
            let resp = StateResponse {
                containers: evt.containers,
                summary,
                progress,
            };
            Some(Ok(json_event("snapshot", &resp)))
        }
    });

    let container_event_stream =
        BroadcastStream::new(container_event_tx.subscribe()).filter_map(|res| async move {
            let evt = res.ok()?;
            Some(Ok(json_event("container-event", &evt)))
        });

    let progress_stream =
        BroadcastStream::new(progress_tx.subscribe()).filter_map(|res| async move {
            let evt = res.ok()?;
            Some(Ok(json_event("update-progress", &evt)))
        });

    let notif_stream = BroadcastStream::new(notif_tx.subscribe()).filter_map(|res| async move {
        let evt = res.ok()?;
        Some(Ok(json_event("notification", &evt)))
    });

    // ── Heartbeat via stream fusion (NOT Sse::keep_alive) ──
    let heartbeat = IntervalStream::new(tokio::time::interval(Duration::from_secs(5)))
        .map(|_| Ok(Event::default().comment("keepalive")));

    let merged = stream::select_all(vec![
        state_stream.boxed(),
        container_event_stream.boxed(),
        progress_stream.boxed(),
        notif_stream.boxed(),
        heartbeat.boxed(),
    ]);

    initial.chain(merged)
}

/// Single multiplexed SSE stream: `GET /api/stream`.
///
/// Authentication is enforced by the shared auth middleware (session cookie);
/// the middleware leaves `text/event-stream` responses untouched. The
/// long-poll `GET /api/state` endpoint is left intact as the fallback transport.
async fn stream_h(
    State(cached_containers): State<CachedContainers>,
    State(progress_cache): State<Arc<Mutex<BatchProgress>>>,
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(progress_tx): State<broadcast::Sender<UpdateProgress>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    State(container_event_tx): State<broadcast::Sender<ContainerEvent>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(
        build_event_stream(
            cached_containers,
            progress_cache,
            tx,
            progress_tx,
            notif_tx,
            container_event_tx,
        )
        .await,
    )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/state", get(state_h))
        .route("/api/stream", get(stream_h))
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
            last_pulled_digest: String::new(),
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
                        last_pulled_digest: String::new(),
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
            last_pulled_digest: String::new(),
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

    #[tokio::test]
    async fn test_state_notify_waits_even_when_cache_populated() {
        /// **Bug**: `state_h` only waits on `state_notify.notified()` when `cached_containers`
        /// is `None` (empty). Once the cache is populated (Some), the handler returns immediately
        /// without waiting, defeating the purpose of long-polling — clients get stale data
        /// until the next state refresh.
        ///
        /// **Expected behavior**: `state_h` should *always* wait up to 30 seconds on the
        /// `state_notify`, regardless of whether the cache is populated. This test verifies
        /// that `state_wait_on_notify` actually blocks.
        use std::time::Instant;

        let containers = vec![make_container("nginx", "running", false)];
        let cached: CachedContainers = Arc::new(RwLock::new(Some(containers)));
        let state_notify = Arc::new(Notify::new());

        let start = Instant::now();
        state_wait_on_notify(&cached, &state_notify).await;
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(95),
            "Expected to wait >=95ms via state_wait_on_notify, but waited only {:?} — \
             hint: the current handler skips waiting when cache is populated",
            elapsed
        );
    }

    // ── /api/stream: merged SSE stream (sse-first, task 1.2) ────

    /// The first item produced by `/api/stream` MUST be a `snapshot` event
    /// carrying a `StateResponse` (containers + summary + progress), emitted
    /// before any incremental event.
    #[tokio::test]
    async fn test_stream_first_event_is_snapshot_state_response() {
        use crate::models::{
            BatchProgress, ContainerEvent, NotifEvent, StateEvent, UpdateProgress,
        };
        use futures::StreamExt;

        let containers = vec![
            make_container("web", "running", false),
            make_container("db", "exited", true),
        ];
        let cached: CachedContainers = Arc::new(RwLock::new(Some(containers)));
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let (tx, _) = broadcast::channel::<StateEvent>(8);
        let (progress_tx, _) = broadcast::channel::<UpdateProgress>(8);
        let (notif_tx, _) = broadcast::channel::<NotifEvent>(8);
        let (container_event_tx, _) = broadcast::channel::<ContainerEvent>(8);

        let mut stream = Box::pin(
            build_event_stream(
                cached,
                progress_cache,
                tx,
                progress_tx,
                notif_tx,
                container_event_tx,
            )
            .await,
        );

        let first = timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("first event within timeout")
            .expect("stream yields an item");
        let first = first.expect("event is Ok");
        let dbg = format!("{:?}", first);

        assert!(
            dbg.contains("snapshot"),
            "first event must be named 'snapshot': {dbg}"
        );
        assert!(
            dbg.contains("containers") && dbg.contains("summary") && dbg.contains("progress"),
            "snapshot payload must be a full StateResponse: {dbg}"
        );
    }
}
