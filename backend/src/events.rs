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

use crate::models::{NotifEvent, StateEvent, UpdateProgress};
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
