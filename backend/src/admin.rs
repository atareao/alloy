use axum::{
    extract::{Path, State},
    response::Json,
    routing::{delete, get},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::*;
use crate::state::AppState;

macro_rules! save_buffered {
    ($path:expr, $data:expr) => {
        crate::workers::json_writer().save($path, $data).await
    };
}

async fn get_alerts_h(
    State(alerts): State<Arc<Mutex<Vec<AlertConfig>>>>,
) -> Json<Vec<AlertConfig>> {
    let list = alerts.lock().await;
    Json(list.clone())
}

async fn create_alert_h(
    State(alerts): State<Arc<Mutex<Vec<AlertConfig>>>>,
    Json(body): Json<CreateAlert>,
) -> Json<AlertConfig> {
    let alert = AlertConfig {
        id: Uuid::new_v4().to_string(),
        r#type: body.r#type,
        container: body.container,
        threshold: body.threshold,
        enabled: body.enabled,
        notify_via: body.notify_via,
    };
    let mut list = alerts.lock().await;
    list.push(alert.clone());
    save_buffered!(FILE_ALERTS, &*list);
    Json(alert)
}

async fn delete_alert_h(
    State(alerts): State<Arc<Mutex<Vec<AlertConfig>>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut list = alerts.lock().await;
    list.retain(|a| a.id != id);
    save_buffered!(FILE_ALERTS, &*list);
    Json(serde_json::json!({"status": "deleted", "id": id}))
}

async fn get_health_checks_h(
    State(health_checks): State<Arc<Mutex<Vec<HealthCheck>>>>,
) -> Json<Vec<HealthCheck>> {
    let list = health_checks.lock().await;
    Json(list.clone())
}

async fn create_health_check_h(
    State(health_checks): State<Arc<Mutex<Vec<HealthCheck>>>>,
    Json(body): Json<CreateHealthCheck>,
) -> Json<HealthCheck> {
    let hc = HealthCheck {
        id: Uuid::new_v4().to_string(),
        r#type: body.r#type,
        target: body.target,
        interval_secs: body.interval_secs,
        container: body.container,
        enabled: body.enabled,
        last_result: None,
    };
    let mut list = health_checks.lock().await;
    list.push(hc.clone());
    save_buffered!(FILE_HEALTH_CHECKS, &*list);
    Json(hc)
}

async fn delete_health_check_h(
    State(health_checks): State<Arc<Mutex<Vec<HealthCheck>>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut list = health_checks.lock().await;
    list.retain(|h| h.id != id);
    save_buffered!(FILE_HEALTH_CHECKS, &*list);
    Json(serde_json::json!({"status": "deleted", "id": id}))
}

async fn get_schedule_h(
    State(schedules): State<Arc<Mutex<Vec<ScheduleTask>>>>,
) -> Json<Vec<ScheduleTask>> {
    let list = schedules.lock().await;
    Json(list.clone())
}

async fn create_schedule_h(
    State(schedules): State<Arc<Mutex<Vec<ScheduleTask>>>>,
    Json(body): Json<CreateSchedule>,
) -> Json<ScheduleTask> {
    let task = ScheduleTask {
        id: Uuid::new_v4().to_string(),
        container: body.container,
        cron: body.cron,
        action: body.action,
        enabled: body.enabled,
    };
    let mut list = schedules.lock().await;
    list.push(task.clone());
    save_buffered!(FILE_SCHEDULES, &*list);
    Json(task)
}

async fn delete_schedule_h(
    State(schedules): State<Arc<Mutex<Vec<ScheduleTask>>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut list = schedules.lock().await;
    list.retain(|s| s.id != id);
    save_buffered!(FILE_SCHEDULES, &*list);
    Json(serde_json::json!({"status": "deleted", "id": id}))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/alerts", get(get_alerts_h).post(create_alert_h))
        .route("/api/alerts/{id}", delete(delete_alert_h))
        .route(
            "/api/health-checks",
            get(get_health_checks_h).post(create_health_check_h),
        )
        .route("/api/health-checks/{id}", delete(delete_health_check_h))
        .route("/api/schedule", get(get_schedule_h).post(create_schedule_h))
        .route("/api/schedule/{id}", delete(delete_schedule_h))
}
