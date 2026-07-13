use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::{future, StreamExt};
use std::convert::Infallible;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[allow(unused_imports)]
use crate::models::{ContainerInfo, NotifEvent, StateEvent, UpdateProgress};
use crate::state::AppState;

#[allow(clippy::type_complexity)]
async fn sse_events_h(
    State(tx): State<broadcast::Sender<StateEvent>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event("containers")
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[allow(clippy::type_complexity)]
async fn sse_updates_h(
    State(tx): State<broadcast::Sender<UpdateProgress>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event("update-progress")
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[allow(clippy::type_complexity)]
async fn sse_notifications_h(
    State(tx): State<broadcast::Sender<NotifEvent>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event("notification")
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/events", get(sse_events_h))
        .route("/api/updates", get(sse_updates_h))
        .route("/api/notifications", get(sse_notifications_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── StateEvent broadcast ─────────────────────────────────

    #[tokio::test]
    async fn test_sse_events_broadcast_send_recv() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let mut rx = tx.subscribe();

        let evt = StateEvent {
            containers: vec![ContainerInfo {
                id: "abc123".into(),
                name: "nginx".into(),
                image: "nginx".into(),
                image_tag: "latest".into(),
                size_mb: 42.5,
                status: "running".into(),
                state: "running".into(),
                has_update: false,
                compose_project: None,
                ports: vec!["0.0.0.0:80:80".into()],
                traefik_url: None,
                registry_url: "https://hub.docker.com/_/nginx".into(),
            }],
        };

        tx.send(evt.clone()).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.containers.len(), 1);
        assert_eq!(received.containers[0].name, "nginx");
        assert_eq!(received.containers[0].state, "running");
    }

    #[tokio::test]
    async fn test_sse_events_broadcast_multiple_containers() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let mut rx = tx.subscribe();

        let evt = StateEvent {
            containers: vec![
                ContainerInfo {
                    id: "1".into(),
                    name: "nginx".into(),
                    image: "nginx".into(),
                    image_tag: "latest".into(),
                    size_mb: 10.0,
                    status: "running".into(),
                    state: "running".into(),
                    has_update: true,
                    compose_project: None,
                    ports: vec![],
                    traefik_url: None,
                    registry_url: String::new(),
                },
                ContainerInfo {
                    id: "2".into(),
                    name: "redis".into(),
                    image: "redis".into(),
                    image_tag: "7".into(),
                    size_mb: 5.0,
                    status: "exited".into(),
                    state: "exited".into(),
                    has_update: false,
                    compose_project: Some("myapp".into()),
                    ports: vec![],
                    traefik_url: None,
                    registry_url: String::new(),
                },
            ],
        };

        tx.send(evt).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.containers.len(), 2);
        assert_eq!(received.containers[0].name, "nginx");
        assert_eq!(received.containers[1].name, "redis");
        assert!(received.containers[0].has_update);
        assert_eq!(
            received.containers[1].compose_project.as_deref(),
            Some("myapp")
        );
    }

    #[tokio::test]
    async fn test_sse_events_multiple_subscribers() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let mut rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();

        let evt = StateEvent {
            containers: vec![ContainerInfo {
                id: "x".into(),
                name: "test".into(),
                image: "test".into(),
                image_tag: "1".into(),
                size_mb: 1.0,
                status: "running".into(),
                state: "running".into(),
                has_update: false,
                compose_project: None,
                ports: vec![],
                traefik_url: None,
                registry_url: String::new(),
            }],
        };

        tx.send(evt.clone()).unwrap();
        let r1 = rx1.recv().await.unwrap();
        let r2 = rx2.recv().await.unwrap();
        assert_eq!(r1.containers[0].name, "test");
        assert_eq!(r2.containers[0].name, "test");
    }

    #[tokio::test]
    async fn test_sse_events_broadcast_lag() {
        let (tx, _) = broadcast::channel::<StateEvent>(2);
        let mut rx = tx.subscribe();

        // Fill the buffer
        for i in 0..3 {
            tx.send(StateEvent {
                containers: vec![ContainerInfo {
                    id: i.to_string(),
                    name: format!("c{}", i),
                    image: "img".into(),
                    image_tag: "latest".into(),
                    size_mb: 1.0,
                    status: "running".into(),
                    state: "running".into(),
                    has_update: false,
                    compose_project: None,
                    ports: vec![],
                    traefik_url: None,
                    registry_url: String::new(),
                }],
            })
            .unwrap();
        }

        // Slow subscriber should get a Lagged error
        let result = rx.recv().await;
        assert!(result.is_err());
        match result {
            Err(broadcast::error::RecvError::Lagged(n)) => {
                assert!(n >= 1);
            }
            _ => panic!("Expected Lagged error"),
        }
    }

    // ── UpdateProgress broadcast ─────────────────────────────

    #[tokio::test]
    async fn test_sse_updates_broadcast() {
        let (tx, _) = broadcast::channel::<UpdateProgress>(16);
        let mut rx = tx.subscribe();

        tx.send(UpdateProgress {
            container: "nginx".into(),
            status: "Pulling".into(),
            done: false,
            error: None,
        })
        .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.container, "nginx");
        assert_eq!(received.status, "Pulling");
        assert!(!received.done);
        assert!(received.error.is_none());
    }

    #[tokio::test]
    async fn test_sse_updates_done_event() {
        let (tx, _) = broadcast::channel::<UpdateProgress>(16);
        let mut rx = tx.subscribe();

        tx.send(UpdateProgress {
            container: "redis".into(),
            status: "Done".into(),
            done: true,
            error: None,
        })
        .unwrap();

        let received = rx.recv().await.unwrap();
        assert!(received.done);
    }

    #[tokio::test]
    async fn test_sse_updates_with_error() {
        let (tx, _) = broadcast::channel::<UpdateProgress>(16);
        let mut rx = tx.subscribe();

        tx.send(UpdateProgress {
            container: "postgres".into(),
            status: "Failed".into(),
            done: true,
            error: Some("connection timeout".into()),
        })
        .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.error.as_deref(), Some("connection timeout"));
    }

    // ── NotifEvent broadcast ─────────────────────────────────

    #[tokio::test]
    async fn test_sse_notifications_broadcast() {
        let (tx, _) = broadcast::channel::<NotifEvent>(16);
        let mut rx = tx.subscribe();

        tx.send(NotifEvent {
            container: "nginx".into(),
            status: "running → exited".into(),
            timestamp: "2026-07-13T19:00:00Z".into(),
        })
        .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.container, "nginx");
        assert_eq!(received.status, "running → exited");
    }

    #[tokio::test]
    async fn test_sse_notifications_multiple_events() {
        let (tx, _) = broadcast::channel::<NotifEvent>(16);
        let mut rx = tx.subscribe();

        for i in 0..3 {
            tx.send(NotifEvent {
                container: format!("c{}", i),
                status: "running".into(),
                timestamp: "now".into(),
            })
            .unwrap();
        }

        for i in 0..3 {
            let received = rx.recv().await.unwrap();
            assert_eq!(received.container, format!("c{}", i));
        }
    }
}
