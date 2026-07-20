pub mod common;
pub mod digest;
pub mod handlers;
pub mod history;

use crate::state::AppState;
use axum::routing::{get, post, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/update/{name}", post(handlers::update_container_h))
        .route("/api/update-all", post(handlers::update_all_h))
        .route("/api/check-update/{name}", post(handlers::check_update_h))
        .route("/api/check-all", post(handlers::check_all_h))
        .route(
            "/api/history",
            get(history::get_history_h).delete(axum::routing::delete(history::delete_history_h)),
        )
}
