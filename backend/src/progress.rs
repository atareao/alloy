use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::StreamExt;

use crate::models::UpdateProgress;

/// Polling-based progress check (legacy, kept for backward compatibility).
pub async fn check_progress_h(
    State(cache): State<Arc<Mutex<HashMap<String, UpdateProgress>>>>,
) -> Json<HashMap<String, UpdateProgress>> {
    let cache = cache.lock().await;
    Json(cache.clone())
}

/// SSE endpoint for update progress.
///
/// Subscribes to the `UpdateProgress` broadcast channel and forwards all events
/// as SSE messages with event name `"progress"` and JSON-encoded data.
async fn updates_sse_h(
    State(tx): State<broadcast::Sender<UpdateProgress>>,
) -> (HeaderMap, Sse<impl futures::Stream<Item = Result<Event, Infallible>>>) {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(progress) => {
            let data = serde_json::to_string(&progress).unwrap_or_default();
            Some(Ok(Event::default().event("progress").data(data)))
        }
        Err(BroadcastStreamRecvError::Lagged(n)) => {
            tracing::warn!("progress SSE lagged by {} messages", n);
            None
        }
    });
    (headers, Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub fn routes() -> axum::Router<crate::state::AppState> {
    axum::Router::new()
        .route("/api/check-progress", axum::routing::get(check_progress_h))
        .route("/api/updates", axum::routing::get(updates_sse_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_progress_returns_cache() {
        let mut cache_map = HashMap::new();
        cache_map.insert(
            "nginx".to_string(),
            UpdateProgress {
                container: "nginx".into(),
                status: "pulling".into(),
                done: false,
                error: None,
                total: 2,
                checked: 1,
                updated: 0,
                errors: 0,
            },
        );
        let cache = Arc::new(Mutex::new(cache_map));
        let response = check_progress_h(State(cache.clone())).await;
        let data = response.0;
        assert_eq!(data.len(), 1);
        assert_eq!(data.get("nginx").unwrap().status, "pulling");
        assert!(!data.get("nginx").unwrap().done);
    }

    #[tokio::test]
    async fn test_check_progress_empty_cache() {
        let cache = Arc::new(Mutex::new(HashMap::new()));
        let response = check_progress_h(State(cache.clone())).await;
        assert!(response.0.is_empty());
    }

    #[tokio::test]
    async fn test_updates_sse_emits_progress_events() {
        let (tx, _rx) = broadcast::channel::<UpdateProgress>(16);
        let progress = UpdateProgress {
            container: "redis".into(),
            status: "done".into(),
            done: true,
            error: None,
            total: 1,
            checked: 1,
            updated: 1,
            errors: 0,
        };

        // Subscribe first, then send
        let mut rx = tx.subscribe();
        tx.send(progress.clone()).unwrap();

        let received = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            rx.recv(),
        )
        .await
        .expect("Should receive event within 1 second")
        .expect("Should not be lagged");

        assert_eq!(received.container, "redis");
        assert_eq!(received.status, "done");
    }
}