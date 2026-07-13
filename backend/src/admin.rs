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
        crate::persistence::json_writer().save($path, $data).await
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
        container: body.container,
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
        target_type: body.target_type,
        cron: body.cron,
        action: body.action,
        enabled: body.enabled,
        notify: body.notify,
        cleanup: body.cleanup,
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
        .route("/api/schedule", get(get_schedule_h).post(create_schedule_h))
        .route("/api/schedule/{id}", delete(delete_schedule_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

    // ── Alerts ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_alerts_empty() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let result: Json<Vec<AlertConfig>> = get_alerts_h(State(alerts)).await;
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_create_alert() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let body = CreateAlert {
            container: "nginx".into(),
            enabled: true,
            notify_via: Vec::new(),
        };
        let result: Json<AlertConfig> = create_alert_h(State(alerts.clone()), Json(body)).await;
        assert_eq!(result.0.container, "nginx");
        assert!(result.0.enabled);
        assert!(Uuid::parse_str(&result.0.id).is_ok());

        // Verify it was stored
        let stored = alerts.lock().await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].container, "nginx");
    }

    #[tokio::test]
    async fn test_create_alert_disabled() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let body = CreateAlert {
            container: "redis".into(),
            enabled: false,
            notify_via: vec!["telegram".into()],
        };
        let result: Json<AlertConfig> = create_alert_h(State(alerts.clone()), Json(body)).await;
        assert_eq!(result.0.container, "redis");
        assert!(!result.0.enabled);
        assert_eq!(result.0.notify_via, vec!["telegram"]);
    }

    #[tokio::test]
    async fn test_delete_alert() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        // Create one
        let body = CreateAlert {
            container: "nginx".into(),
            enabled: true,
            notify_via: Vec::new(),
        };
        let created: Json<AlertConfig> = create_alert_h(State(alerts.clone()), Json(body)).await;
        let id = created.0.id.clone();

        // Delete it
        let result: Json<serde_json::Value> =
            delete_alert_h(State(alerts.clone()), Path(id.clone())).await;
        assert_eq!(result.0["status"], "deleted");

        // Verify it's gone
        let stored = alerts.lock().await;
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn test_delete_alert_nonexistent() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> =
            delete_alert_h(State(alerts.clone()), Path("nonexistent".into())).await;
        assert_eq!(result.0["status"], "deleted");
    }

    #[tokio::test]
    async fn test_get_alerts_multiple() {
        let alerts = Arc::new(Mutex::new(Vec::new()));
        let containers = vec!["nginx", "redis", "postgres"];
        for c in &containers {
            let body = CreateAlert {
                container: c.to_string(),
                enabled: true,
                notify_via: Vec::new(),
            };
            let _ = create_alert_h(State(alerts.clone()), Json(body)).await;
        }
        let result: Json<Vec<AlertConfig>> = get_alerts_h(State(alerts.clone())).await;
        assert_eq!(result.0.len(), 3);
        assert_eq!(result.0[0].container, "nginx");
        assert_eq!(result.0[1].container, "redis");
        assert_eq!(result.0[2].container, "postgres");
    }

    // ── Schedules ────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_schedules_empty() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        let result: Json<Vec<ScheduleTask>> = get_schedule_h(State(schedules)).await;
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_create_schedule() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        let body = CreateSchedule {
            container: "nginx".into(),
            target_type: default_target_type(),
            cron: "0 3 * * *".into(),
            action: "restart".into(),
            enabled: true,
            notify: true,
            cleanup: default_cleanup(),
        };
        let result: Json<ScheduleTask> =
            create_schedule_h(State(schedules.clone()), Json(body)).await;
        assert_eq!(result.0.container, "nginx");
        assert_eq!(result.0.cron, "0 3 * * *");
        assert_eq!(result.0.action, "restart");
        assert!(result.0.enabled);
        assert!(result.0.notify);
        assert_eq!(result.0.target_type, "container");
        assert_eq!(result.0.cleanup, "none");
        assert!(Uuid::parse_str(&result.0.id).is_ok());

        // Verify stored
        let stored = schedules.lock().await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].container, "nginx");
    }

    #[tokio::test]
    async fn test_create_schedule_disabled_no_notify() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        let body = CreateSchedule {
            container: "postgres".into(),
            target_type: "container".into(),
            cron: "0 6 * * 0".into(),
            action: "stop".into(),
            enabled: false,
            notify: false,
            cleanup: "volume".into(),
        };
        let result: Json<ScheduleTask> =
            create_schedule_h(State(schedules.clone()), Json(body)).await;
        assert_eq!(result.0.container, "postgres");
        assert!(!result.0.enabled);
        assert!(!result.0.notify);
        assert_eq!(result.0.cleanup, "volume");
    }

    #[tokio::test]
    async fn test_delete_schedule() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        let body = CreateSchedule {
            container: "nginx".into(),
            target_type: default_target_type(),
            cron: "0 3 * * *".into(),
            action: "restart".into(),
            enabled: true,
            notify: false,
            cleanup: default_cleanup(),
        };
        let created: Json<ScheduleTask> =
            create_schedule_h(State(schedules.clone()), Json(body)).await;
        let id = created.0.id.clone();

        let result: Json<serde_json::Value> =
            delete_schedule_h(State(schedules.clone()), Path(id)).await;
        assert_eq!(result.0["status"], "deleted");

        let stored = schedules.lock().await;
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn test_delete_schedule_nonexistent() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> =
            delete_schedule_h(State(schedules.clone()), Path("nonexistent".into())).await;
        assert_eq!(result.0["status"], "deleted");
    }

    #[tokio::test]
    async fn test_get_schedules_multiple() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        for i in 0..3 {
            let body = CreateSchedule {
                container: format!("container_{}", i),
                target_type: default_target_type(),
                cron: "0 3 * * *".into(),
                action: "restart".into(),
                enabled: true,
                notify: false,
                cleanup: default_cleanup(),
            };
            let _ = create_schedule_h(State(schedules.clone()), Json(body)).await;
        }
        let result: Json<Vec<ScheduleTask>> = get_schedule_h(State(schedules.clone())).await;
        assert_eq!(result.0.len(), 3);
    }

    // ── Cross-contamination ──────────────────────────────────

    #[tokio::test]
    async fn test_alerts_and_schedules_isolated() {
        let alerts: Arc<Mutex<Vec<AlertConfig>>> = Arc::new(Mutex::new(Vec::new()));
        let schedules: Arc<Mutex<Vec<ScheduleTask>>> = Arc::new(Mutex::new(Vec::new()));

        // Create alert
        let body = CreateAlert {
            container: "nginx".into(),
            enabled: true,
            notify_via: Vec::new(),
        };
        let _ = create_alert_h(State(alerts.clone()), Json(body)).await;

        // Verify alerts has it, schedules doesn't
        assert_eq!(alerts.lock().await.len(), 1);
        assert!(schedules.lock().await.is_empty());
    }
}
