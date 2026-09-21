use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::{future, stream, StreamExt};
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
    tracing::info!("📡 SSE /api/updates: cliente conectado");
    let stream = BroadcastStream::new(tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => {
            tracing::info!(
                "📡 SSE /api/updates: enviando evento container={} done={} checked={} total={} updated={} errors={} status={}",
                evt.container,
                evt.done,
                evt.checked,
                evt.total,
                evt.updated,
                evt.errors,
                evt.status
            );
            future::ready(Some(Ok(Event::default()
                .event("update-progress")
                .json_data(evt)
                .unwrap())))
        }
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

#[allow(clippy::type_complexity)]
async fn sse_stream_h(
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let containers_stream = BroadcastStream::new(tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event("containers")
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    });

    let update_stream = BroadcastStream::new(update_tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event("update-progress")
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    });

    let notif_stream = BroadcastStream::new(notif_tx.subscribe()).filter_map(|r| match r {
        Ok(evt) => future::ready(Some(Ok(Event::default()
            .event("notification")
            .json_data(evt)
            .unwrap()))),
        Err(_) => future::ready(None),
    });

    let merged = stream::select_all(vec![
        containers_stream.boxed(),
        update_stream.boxed(),
        notif_stream.boxed(),
    ]);

    Sse::new(merged).keep_alive(KeepAlive::default())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/events", get(sse_events_h))
        .route("/api/updates", get(sse_updates_h))
        .route("/api/notifications", get(sse_notifications_h))
        .route("/api/stream", get(sse_stream_h))
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
                updating: false,
                registry_url: "https://hub.docker.com/_/nginx".into(),
                last_check: None,
                next_check: None,
                last_remote_digest: String::new(),
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
                    updating: false,
                    registry_url: String::new(),
                    last_check: None,
                    next_check: None,
                    last_remote_digest: String::new(),
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
                    updating: false,
                    registry_url: String::new(),
                    last_check: None,
                    next_check: None,
                    last_remote_digest: String::new(),
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
                updating: false,
                registry_url: String::new(),
                last_check: None,
                next_check: None,
                last_remote_digest: String::new(),
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
                    updating: false,
                    registry_url: String::new(),
                    last_check: None,
                    next_check: None,
                    last_remote_digest: String::new(),
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
            total: 0,
            checked: 0,
            updated: 0,
            errors: 0,
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
            total: 0,
            checked: 0,
            updated: 0,
            errors: 0,
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
            total: 0,
            checked: 0,
            updated: 0,
            errors: 0,
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

    // ── Multiplexed SSE stream ────────────────────────────────

    #[tokio::test]
    async fn test_sse_stream_merges_all_events() {
        let (tx, _) = broadcast::channel::<StateEvent>(16);
        let (update_tx, _) = broadcast::channel::<UpdateProgress>(16);
        let (notif_tx, _) = broadcast::channel::<NotifEvent>(16);

        // Build sub-streams matching the handler logic
        let containers_stream = BroadcastStream::new(tx.subscribe()).filter_map(|r| match r {
            Ok(evt) => future::ready(Some(
                Event::default().event("containers").json_data(evt).unwrap(),
            )),
            Err(_) => future::ready(None),
        });

        let update_stream = BroadcastStream::new(update_tx.subscribe()).filter_map(|r| match r {
            Ok(evt) => future::ready(Some(
                Event::default()
                    .event("update-progress")
                    .json_data(evt)
                    .unwrap(),
            )),
            Err(_) => future::ready(None),
        });

        let notif_stream = BroadcastStream::new(notif_tx.subscribe()).filter_map(|r| match r {
            Ok(evt) => future::ready(Some(
                Event::default()
                    .event("notification")
                    .json_data(evt)
                    .unwrap(),
            )),
            Err(_) => future::ready(None),
        });

        // Merge with select_all (same as handler)
        let merged = stream::select_all(vec![
            containers_stream.boxed(),
            update_stream.boxed(),
            notif_stream.boxed(),
        ]);

        // Send one event of each type
        tx.send(StateEvent {
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
                updating: false,
                registry_url: String::new(),
                last_check: None,
                next_check: None,
                last_remote_digest: String::new(),
            }],
        })
        .unwrap();

        update_tx
            .send(UpdateProgress {
                container: "nginx".into(),
                status: "Pulling".into(),
                done: false,
                error: None,
                total: 3,
                checked: 1,
                updated: 0,
                errors: 0,
            })
            .unwrap();

        notif_tx
            .send(NotifEvent {
                container: "nginx".into(),
                status: "restarting".into(),
                timestamp: "now".into(),
            })
            .unwrap();

        // Collect 3 events from the merged stream (one of each type)
        let events: Vec<Event> = merged.take(3).collect().await;

        assert_eq!(events.len(), 3, "should receive exactly 3 events");

        // Convert events to Debug text to verify event names
        let text: String = events.iter().map(|e| format!("{e:?}")).collect();

        assert!(
            text.contains(r#"event: containers\n"#),
            "Debug text should contain 'event: containers'\nGot: {:?}",
            text
        );
        assert!(
            text.contains(r#"event: update-progress\n"#),
            "Debug text should contain 'event: update-progress'\nGot: {:?}",
            text
        );
        assert!(
            text.contains(r#"event: notification\n"#),
            "Debug text should contain 'event: notification'\nGot: {:?}",
            text
        );
    }
}
