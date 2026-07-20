use axum::{
    extract::State,
    response::Json,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::state::AppState;

// ── Update Check Config ──────────────────────────────────────

async fn get_update_check_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<UpdateCheckConfig> {
    let s = settings.lock().await;
    Json(UpdateCheckConfig {
        cron: s
            .update_check_cron
            .clone()
            .unwrap_or_else(|| "0 */6 * * *".into()),
        enabled: s.update_check_enabled.unwrap_or(false),
        notify: s.update_check_notify.unwrap_or(false),
    })
}

async fn put_update_check_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<UpdateCheckConfig>,
) -> Json<UpdateCheckConfig> {
    let mut s = settings.lock().await;
    s.update_check_cron = Some(body.cron.clone());
    s.update_check_enabled = Some(body.enabled);
    s.update_check_notify = Some(body.notify);
    let conn = db_pool.get().await.unwrap();
    let _ = db::save_settings(&conn.lock().unwrap(), &s);
    Json(body)
}

// ── Update Policies ─────────────────────────────────────────

async fn get_update_policies_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
) -> Json<Vec<UpdatePolicy>> {
    let list = policies.lock().await;
    Json(list.clone())
}

async fn put_update_policy_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    State(db_pool): State<DbPool>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<UpdatePolicyReq>,
) -> Json<UpdatePolicy> {
    let policy = UpdatePolicy {
        container: name.clone(),
        action: body.action,
        cleanup_old_image: body.cleanup_old_image,
        rollback_on_failure: body.rollback_on_failure,
        notify_events: false,
    };
    {
        let mut list = policies.lock().await;
        if let Some(existing) = list.iter_mut().find(|p| p.container == name) {
            existing.action = policy.action.clone();
            existing.cleanup_old_image = policy.cleanup_old_image;
            existing.rollback_on_failure = policy.rollback_on_failure;
        } else {
            list.push(policy.clone());
        }
    }
    let conn = db_pool.get().await.unwrap();
    let _ = db::save_update_policy(&conn.lock().unwrap(), &policy);
    Json(policy)
}

async fn delete_update_policy_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    State(db_pool): State<DbPool>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    {
        let mut list = policies.lock().await;
        list.retain(|p| p.container != name);
    }
    let conn = db_pool.get().await.unwrap();
    let _ = db::delete_update_policy(&conn.lock().unwrap(), &name);
    Json(serde_json::json!({"status": "deleted", "container": name}))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DefaultUpdatePolicy {
    action: String,
    cleanup_old_image: bool,
    rollback_on_failure: bool,
}

async fn get_default_update_policy_h(
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<DefaultUpdatePolicy> {
    let s = settings.lock().await;
    Json(DefaultUpdatePolicy {
        action: s
            .default_update_action
            .clone()
            .unwrap_or_else(|| "pull-restart".into()),
        cleanup_old_image: s.default_cleanup_old_image.unwrap_or(false),
        rollback_on_failure: s.default_rollback_on_failure.unwrap_or(false),
    })
}

async fn put_default_update_policy_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<DefaultUpdatePolicy>,
) -> Json<DefaultUpdatePolicy> {
    let mut s = settings.lock().await;
    s.default_update_action = Some(body.action.clone());
    s.default_cleanup_old_image = Some(body.cleanup_old_image);
    s.default_rollback_on_failure = Some(body.rollback_on_failure);
    let conn = db_pool.get().await.unwrap();
    let _ = db::save_settings(&conn.lock().unwrap(), &s);
    Json(body)
}

// ── Export / Import ──────────────────────────────────────────

async fn export_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
) -> Json<serde_json::Value> {
    let sett = settings.lock().await;
    let hist = update_history.lock().await;
    let conn = db_pool.get().await.unwrap();
    let _ = db::save_settings(&conn.lock().unwrap(), &sett);
    Json(serde_json::json!({
        "version": 1,
        "exported_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "settings": *sett,
        "update_history": *hist,
    }))
}

#[derive(serde::Deserialize)]
struct ImportPayload {
    settings: Settings,
}

async fn import_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<ImportPayload>,
) -> Json<serde_json::Value> {
    {
        let mut st = settings.lock().await;
        *st = body.settings;
        let conn = db_pool.get().await.unwrap();
        let _ = db::save_settings(&conn.lock().unwrap(), &st);
    }
    Json(serde_json::json!({"status": "imported"}))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/update-check/config",
            get(get_update_check_config_h).put(put_update_check_config_h),
        )
        .route("/api/update-policies", get(get_update_policies_h))
        .route(
            "/api/update-policies/default",
            get(get_default_update_policy_h).put(put_default_update_policy_h),
        )
        .route(
            "/api/update-policies/{name}",
            put(put_update_policy_h).delete(delete_update_policy_h),
        )
        .route("/api/admin/export", get(export_config_h))
        .route("/api/admin/import", post(import_config_h))
}

use axum::routing::post;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

    // ── Update Check Config ─────────────────────────────────

    #[tokio::test]
    async fn test_get_update_check_config_defaults() {
        let settings = Arc::new(Mutex::new(Settings::default()));
        let result: Json<UpdateCheckConfig> = get_update_check_config_h(State(settings)).await;
        assert_eq!(result.0.cron, "0 */6 * * *");
        assert!(!result.0.enabled);
        assert!(!result.0.notify);
    }

    #[tokio::test]
    async fn test_put_update_check_config() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let config = UpdateCheckConfig {
            cron: "0 0 * * *".into(),
            enabled: true,
            notify: true,
        };
        let result: Json<UpdateCheckConfig> =
            put_update_check_config_h(State(settings.clone()), State(db_pool), Json(config)).await;
        assert_eq!(result.0.cron, "0 0 * * *");
        assert!(result.0.enabled);
        assert!(result.0.notify);
        let s = settings.lock().await;
        assert_eq!(s.update_check_cron.as_deref(), Some("0 0 * * *"));
        assert_eq!(s.update_check_enabled, Some(true));
    }

    // ── Update Policies ─────────────────────────────────────

    #[tokio::test]
    async fn test_get_update_policies_empty() {
        let policies = Arc::new(Mutex::new(Vec::new()));
        let result: Json<Vec<UpdatePolicy>> = get_update_policies_h(State(policies)).await;
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_put_update_policy() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(Vec::new()));
        let req = UpdatePolicyReq {
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: true,
            notify_events: false,
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
            Json(req),
        )
        .await;
        assert_eq!(result.0.container, "nginx");
        assert_eq!(result.0.action, UpdateAction::PullRestart);
        assert!(result.0.cleanup_old_image);
        assert!(result.0.rollback_on_failure);
        let list = policies.lock().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].action, UpdateAction::PullRestart);
    }

    #[tokio::test]
    async fn test_put_update_policy_overwrites() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(vec![UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::None,
            cleanup_old_image: false,
            rollback_on_failure: false,
            notify_events: false,
        }]));
        let req = UpdatePolicyReq {
            action: UpdateAction::Pull,
            cleanup_old_image: false,
            rollback_on_failure: false,
            notify_events: false,
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
            Json(req),
        )
        .await;
        assert_eq!(result.0.action, UpdateAction::Pull);
        let list = policies.lock().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].action, UpdateAction::Pull);
    }

    #[tokio::test]
    async fn test_delete_update_policy() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(vec![UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: false,
            notify_events: false,
        }]));
        let result: Json<serde_json::Value> = delete_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
        )
        .await;
        assert_eq!(result.0["status"], "deleted");
        let list = policies.lock().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_delete_update_policy_nonexistent() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> = delete_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nonexistent".into()),
        )
        .await;
        assert_eq!(result.0["status"], "deleted");
    }

    #[tokio::test]
    async fn test_get_update_policies_multiple() {
        let policies = Arc::new(Mutex::new(vec![
            UpdatePolicy {
                container: "nginx".into(),
                action: UpdateAction::Pull,
                cleanup_old_image: false,
                rollback_on_failure: false,
                notify_events: false,
            },
            UpdatePolicy {
                container: "redis".into(),
                action: UpdateAction::None,
                cleanup_old_image: false,
                rollback_on_failure: false,
                notify_events: false,
            },
        ]));
        let result: Json<Vec<UpdatePolicy>> = get_update_policies_h(State(policies)).await;
        assert_eq!(result.0.len(), 2);
    }

    // ── Export / Import ──────────────────────────────────────

    #[tokio::test]
    async fn test_export_config_empty() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));

        let result: Json<serde_json::Value> =
            export_config_h(State(settings), State(update_history), State(db_pool)).await;

        assert_eq!(result.0["version"], 1);
        assert!(result.0["exported_at"].is_string());
        assert!(result.0["settings"].is_object());
    }

    #[tokio::test]
    async fn test_import_config_replaces_state() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings {
                telegram_token: None,
                telegram_chat_id: Some("123".into()),
                ..Default::default()
            },
        };

        let _: Json<serde_json::Value> =
            import_config_h(State(settings.clone()), State(db_pool), Json(payload)).await;

        let st = settings.lock().await;
        assert!(st.telegram_token.is_none());
        assert_eq!(st.telegram_chat_id.as_deref(), Some("123"));
    }

    #[tokio::test]
    async fn test_import_config_overwrites_existing() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings::default(),
        };

        let _: Json<serde_json::Value> =
            import_config_h(State(settings.clone()), State(db_pool), Json(payload)).await;
    }
}
