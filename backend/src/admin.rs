use axum::{
    extract::State,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db;
use crate::models::*;
use crate::state::AppState;

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
    let conn = db::global().lock().await;
    let _ = db::save_schedule(&conn, &task);
    Json(task)
}

async fn delete_schedule_h(
    State(schedules): State<Arc<Mutex<Vec<ScheduleTask>>>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let mut list = schedules.lock().await;
    list.retain(|s| s.id != id);
    let conn = db::global().lock().await;
    let _ = db::delete_schedule(&conn, &id);
    Json(serde_json::json!({"status": "deleted", "id": id}))
}

async fn export_config_h(
    State(schedules): State<Arc<Mutex<Vec<ScheduleTask>>>>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<serde_json::Value> {
    let s = schedules.lock().await;
    let sett = settings.lock().await;
    let hist = update_history.lock().await;
    let conn = db::global().lock().await;
    let _ = (|| -> Result<(), Box<dyn std::error::Error>> {
        for sched in s.iter() {
            db::save_schedule(&conn, sched)?;
        }
        db::save_settings(&conn, &sett)?;
        Ok(())
    })();
    Json(serde_json::json!({
        "version": 1,
        "exported_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "schedules": *s,
        "settings": *sett,
        "update_history": *hist,
    }))
}

#[derive(serde::Deserialize)]
struct ImportPayload {
    schedules: Vec<ScheduleTask>,
    settings: Settings,
}

async fn import_config_h(
    State(schedules): State<Arc<Mutex<Vec<ScheduleTask>>>>,
    State(settings): State<Arc<Mutex<Settings>>>,
    Json(body): Json<ImportPayload>,
) -> Json<serde_json::Value> {
    {
        let mut s = schedules.lock().await;
        *s = body.schedules;
        let conn = db::global().lock().await;
        for sched in s.iter() {
            let _ = db::save_schedule(&conn, sched);
        }
    }
    {
        let mut st = settings.lock().await;
        *st = body.settings;
        let conn = db::global().lock().await;
        let _ = db::save_settings(&conn, &st);
    }
    Json(serde_json::json!({"status": "imported"}))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/schedule", get(get_schedule_h).post(create_schedule_h))
        .route("/api/schedule/{id}", delete(delete_schedule_h))
        .route("/api/admin/export", get(export_config_h))
        .route("/api/admin/import", post(import_config_h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

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
            delete_schedule_h(State(schedules.clone()), axum::extract::Path(id)).await;
        assert_eq!(result.0["status"], "deleted");

        let stored = schedules.lock().await;
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn test_delete_schedule_nonexistent() {
        let schedules = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> = delete_schedule_h(
            State(schedules.clone()),
            axum::extract::Path("nonexistent".into()),
        )
        .await;
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

    // ── Export / Import ──────────────────────────────────────

    #[tokio::test]
    async fn test_export_config_empty() {
        let schedules: Arc<Mutex<Vec<ScheduleTask>>> = Arc::new(Mutex::new(Vec::new()));
        let settings: Arc<Mutex<Settings>> = Arc::new(Mutex::new(Settings::default()));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));

        let result: Json<serde_json::Value> =
            export_config_h(State(schedules), State(settings), State(update_history)).await;

        assert_eq!(result.0["version"], 1);
        assert!(result.0["exported_at"].is_string());
        assert!(result.0["schedules"].as_array().unwrap().is_empty());
        assert!(result.0["update_history"].as_array().unwrap().is_empty());
        assert!(result.0["settings"].is_object());
    }

    #[tokio::test]
    async fn test_export_config_with_data() {
        let schedules: Arc<Mutex<Vec<ScheduleTask>>> = Arc::new(Mutex::new(Vec::new()));
        let settings: Arc<Mutex<Settings>> = Arc::new(Mutex::new(Settings {
            auto_update_enabled: Some(true),
            telegram_token: Some("tok".into()),
            ..Default::default()
        }));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));

        let result: Json<serde_json::Value> =
            export_config_h(State(schedules), State(settings), State(update_history)).await;

        let exported_settings = &result.0["settings"];
        assert_eq!(exported_settings["auto_update_enabled"], true);
        assert_eq!(exported_settings["telegram_token"], "tok");
    }

    #[tokio::test]
    async fn test_import_config_replaces_state() {
        let schedules: Arc<Mutex<Vec<ScheduleTask>>> = Arc::new(Mutex::new(Vec::new()));
        let settings: Arc<Mutex<Settings>> = Arc::new(Mutex::new(Settings::default()));

        // Import new data
        let payload = ImportPayload {
            schedules: vec![ScheduleTask {
                id: "sched-1".into(),
                container: "backup".into(),
                target_type: "container".into(),
                cron: "0 3 * * *".into(),
                action: "restart".into(),
                enabled: true,
                notify: false,
                cleanup: "none".into(),
            }],
            settings: Settings {
                auto_update_enabled: Some(false),
                telegram_token: None,
                telegram_chat_id: Some("123".into()),
                ..Default::default()
            },
        };

        let _: Json<serde_json::Value> = import_config_h(
            State(schedules.clone()),
            State(settings.clone()),
            Json(payload),
        )
        .await;

        // Verify state was replaced
        {
            let s = schedules.lock().await;
            assert_eq!(s.len(), 1);
            assert_eq!(s[0].container, "backup");
        }
        {
            let st = settings.lock().await;
            assert_eq!(st.auto_update_enabled, Some(false));
            assert!(st.telegram_token.is_none());
            assert_eq!(st.telegram_chat_id.as_deref(), Some("123"));
        }
    }

    #[tokio::test]
    async fn test_import_config_overwrites_existing() {
        let schedules: Arc<Mutex<Vec<ScheduleTask>>> = Arc::new(Mutex::new(Vec::new()));
        let settings: Arc<Mutex<Settings>> = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            schedules: vec![],
            settings: Settings::default(),
        };

        let _: Json<serde_json::Value> = import_config_h(
            State(schedules.clone()),
            State(settings.clone()),
            Json(payload),
        )
        .await;

        // Should have replaced with empty
        assert!(schedules.lock().await.is_empty());
    }
}
