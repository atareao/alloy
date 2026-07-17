use axum::{
    extract::State,
    response::Json,
    routing::{get, put},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
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
    Json(body): Json<UpdateCheckConfig>,
) -> Json<UpdateCheckConfig> {
    let mut s = settings.lock().await;
    s.update_check_cron = Some(body.cron.clone());
    s.update_check_enabled = Some(body.enabled);
    s.update_check_notify = Some(body.notify);
    let conn = db::global().lock().await;
    let _ = db::save_settings(&conn, &s);
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
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<UpdatePolicyReq>,
) -> Json<UpdatePolicy> {
    let policy = UpdatePolicy {
        container: name.clone(),
        action: body.action,
        cleanup_old_image: body.cleanup_old_image,
        rollback_on_failure: body.rollback_on_failure,
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
    let conn = db::global().lock().await;
    let _ = db::save_update_policy(&conn, &policy);
    Json(policy)
}

async fn delete_update_policy_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    {
        let mut list = policies.lock().await;
        list.retain(|p| p.container != name);
    }
    let conn = db::global().lock().await;
    let _ = db::delete_update_policy(&conn, &name);
    Json(serde_json::json!({"status": "deleted", "container": name}))
}

// ── Export / Import ──────────────────────────────────────────

async fn export_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<serde_json::Value> {
    let sett = settings.lock().await;
    let hist = update_history.lock().await;
    let conn = db::global().lock().await;
    let _ = db::save_settings(&conn, &sett);
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
    Json(body): Json<ImportPayload>,
) -> Json<serde_json::Value> {
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
        .route(
            "/api/update-check/config",
            get(get_update_check_config_h).put(put_update_check_config_h),
        )
        .route("/api/update-policies", get(get_update_policies_h))
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
        let settings = Arc::new(Mutex::new(Settings::default()));
        let config = UpdateCheckConfig {
            cron: "0 0 * * *".into(),
            enabled: true,
            notify: true,
        };
        let result: Json<UpdateCheckConfig> =
            put_update_check_config_h(State(settings.clone()), Json(config)).await;
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
        let policies = Arc::new(Mutex::new(Vec::new()));
        let req = UpdatePolicyReq {
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: true,
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
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
        let policies = Arc::new(Mutex::new(vec![UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::None,
            cleanup_old_image: false,
            rollback_on_failure: false,
        }]));
        let req = UpdatePolicyReq {
            action: UpdateAction::Pull,
            cleanup_old_image: false,
            rollback_on_failure: false,
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
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
        let policies = Arc::new(Mutex::new(vec![UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: false,
        }]));
        let result: Json<serde_json::Value> =
            delete_update_policy_h(State(policies.clone()), axum::extract::Path("nginx".into()))
                .await;
        assert_eq!(result.0["status"], "deleted");
        let list = policies.lock().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_delete_update_policy_nonexistent() {
        let policies = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> = delete_update_policy_h(
            State(policies.clone()),
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
            },
            UpdatePolicy {
                container: "redis".into(),
                action: UpdateAction::None,
                cleanup_old_image: false,
                rollback_on_failure: false,
            },
        ]));
        let result: Json<Vec<UpdatePolicy>> = get_update_policies_h(State(policies)).await;
        assert_eq!(result.0.len(), 2);
    }

    // ── Export / Import ──────────────────────────────────────

    #[tokio::test]
    async fn test_export_config_empty() {
        let settings = Arc::new(Mutex::new(Settings::default()));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));

        let result: Json<serde_json::Value> =
            export_config_h(State(settings), State(update_history)).await;

        assert_eq!(result.0["version"], 1);
        assert!(result.0["exported_at"].is_string());
        assert!(result.0["settings"].is_object());
    }

    #[tokio::test]
    async fn test_import_config_replaces_state() {
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings {
                auto_update_enabled: Some(false),
                telegram_token: None,
                telegram_chat_id: Some("123".into()),
                ..Default::default()
            },
        };

        let _: Json<serde_json::Value> =
            import_config_h(State(settings.clone()), Json(payload)).await;

        let st = settings.lock().await;
        assert_eq!(st.auto_update_enabled, Some(false));
        assert!(st.telegram_token.is_none());
        assert_eq!(st.telegram_chat_id.as_deref(), Some("123"));
    }

    #[tokio::test]
    async fn test_import_config_overwrites_existing() {
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings::default(),
        };

        let _: Json<serde_json::Value> =
            import_config_h(State(settings.clone()), Json(payload)).await;
    }
}
