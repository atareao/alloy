use axum::{extract::State, Json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::UpdateProgress;

pub async fn check_progress_h(
    State(cache): State<Arc<Mutex<HashMap<String, UpdateProgress>>>>,
) -> Json<HashMap<String, UpdateProgress>> {
    let cache = cache.lock().await;
    Json(cache.clone())
}

pub fn routes() -> axum::Router<crate::state::AppState> {
    axum::Router::new().route("/api/check-progress", axum::routing::get(check_progress_h))
}
