use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::Settings;
use crate::state::http_client;

/// Resolve telegram token from settings
fn tg_token(settings: &Settings) -> Option<&str> {
    settings.telegram_token.as_deref()
}

fn tg_chat_id(settings: &Settings) -> Option<&str> {
    settings.telegram_chat_id.as_deref()
}

fn mx_homeserver(settings: &Settings) -> Option<&str> {
    settings.matrix_homeserver.as_deref()
}

fn mx_token(settings: &Settings) -> Option<&str> {
    settings.matrix_token.as_deref()
}

fn mx_room(settings: &Settings) -> Option<&str> {
    settings.matrix_room.as_deref()
}

/// Returns `Ok(())` on success, `Err(message)` on failure.
pub async fn notify_telegram(
    settings: &Settings,
    container: &str,
    status: &str,
) -> Result<(), String> {
    let (Some(token), Some(chat_id)) = (tg_token(settings), tg_chat_id(settings)) else {
        return Err("Telegram no configurado: falta token o chat_id".into());
    };
    let body = serde_json::json!({"chat_id": chat_id, "text": format!("🪐 *Alloy*\n*{}*: {}", container, status), "parse_mode": "Markdown"});
    match http_client()
        .post(format!("https://api.telegram.org/bot{}/sendMessage", token))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!("Telegram: ✅ notificación enviada");
                Ok(())
            } else {
                let status_code = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let err = format!("Telegram: HTTP {} - {}", status_code, text);
                tracing::error!("{}", err);
                Err(err)
            }
        }
        Err(e) => {
            let err = format!("Telegram: error de conexión: {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
}

/// Returns `Ok(())` on success, `Err(message)` on failure.
pub async fn notify_matrix(
    settings: &Settings,
    container: &str,
    status: &str,
) -> Result<(), String> {
    let (hs, token, room) = (
        mx_homeserver(settings),
        mx_token(settings),
        mx_room(settings),
    );
    tracing::info!(
        "notify_matrix: hs={:?} token_set={} room={:?}",
        hs,
        token.is_some(),
        room
    );
    let (Some(hs), Some(token), Some(room)) = (hs, token, room) else {
        let err = "Matrix no configurado: faltan homeserver, token o room".to_string();
        tracing::warn!("notify_matrix: 🚫 {}", err);
        return Err(err);
    };
    let body =
        serde_json::json!({"msgtype": "m.notice", "body": format!("🐳 {}: {}", container, status)});
    let url = format!(
        "{}/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
        hs.trim_end_matches('/'),
        room,
        uuid::Uuid::new_v4()
    );
    match http_client()
        .put(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!("Matrix: ✅ notificación enviada a {}", hs);
                Ok(())
            } else {
                let status_code = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                let err = format!("Matrix: HTTP {} - {}", status_code, body_text);
                tracing::error!("{}", err);
                Err(err)
            }
        }
        Err(e) => {
            let err = format!("Matrix: error de conexión: {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
}

/// Returns `Ok(())` on success, `Err(message)` on failure.
pub async fn notify_webhook(
    settings: &Settings,
    container: &str,
    status: &str,
) -> Result<(), String> {
    let url = settings.webhook_url.as_deref();
    let Some(url) = url else {
        return Err("Webhook no configurado: falta URL".into());
    };
    let body = serde_json::json!({
        "event": "container_status",
        "container": container,
        "status": status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "source": "alloy"
    });
    match http_client().post(url).json(&body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!("Webhook: ✅ notificación enviada");
                Ok(())
            } else {
                let s = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let err = format!("Webhook: HTTP {} - {}", s, text);
                tracing::error!("{}", err);
                Err(err)
            }
        }
        Err(e) => {
            let err = format!("Webhook: error de conexión: {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
}

pub async fn notify_all(settings: &Arc<Mutex<Settings>>, container: &str, status: &str) {
    let s = settings.lock().await;
    let _ = tokio::join!(
        notify_telegram(&s, container, status),
        notify_matrix(&s, container, status),
        notify_webhook(&s, container, status)
    );
}

// ── Test notification endpoint ──────────────────────────────

use axum::{extract::State, response::Json, routing::post, Router};

use crate::models::TestNotificationReq;
use crate::state::AppState;

async fn test_notification_h(
    State(settings): State<std::sync::Arc<tokio::sync::Mutex<crate::models::Settings>>>,
    Json(body): Json<TestNotificationReq>,
) -> Result<Json<serde_json::Value>, crate::models::AppError> {
    let s = settings.lock().await;
    match body.channel.as_str() {
        "telegram" => {
            let msg = "🧪 Test de notificación desde Alloy — Telegram funciona correctamente ✅";
            match notify_telegram(&s, "Alloy Test", msg).await {
                Ok(()) => Ok(Json(serde_json::json!({
                    "status": "ok",
                    "channel": "telegram",
                    "message": "enviado"
                }))),
                Err(e) => Err(crate::models::AppError::Internal(e)),
            }
        }
        "matrix" => {
            let msg = "🧪 Test de notificación desde Alloy — Matrix funciona correctamente ✅";
            match notify_matrix(&s, "Alloy Test", msg).await {
                Ok(()) => Ok(Json(serde_json::json!({
                    "status": "ok",
                    "channel": "matrix",
                    "message": "enviado"
                }))),
                Err(e) => Err(crate::models::AppError::Internal(e)),
            }
        }
        other => Err(crate::models::AppError::BadRequest(format!(
            "Canal desconocido: '{}'. Usa 'telegram' o 'matrix'.",
            other
        ))),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/notifications/test", post(test_notification_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_settings() -> Settings {
        Settings::default()
    }

    fn settings_with_tg(token: &str, chat_id: &str) -> Settings {
        Settings {
            telegram_token: Some(token.into()),
            telegram_chat_id: Some(chat_id.into()),
            ..Default::default()
        }
    }

    fn settings_with_mx(hs: &str, token: &str, room: &str) -> Settings {
        Settings {
            matrix_homeserver: Some(hs.into()),
            matrix_token: Some(token.into()),
            matrix_room: Some(room.into()),
            ..Default::default()
        }
    }

    // ── tg_token ────────────────────────────────────────────

    #[test]
    fn test_tg_token_both_none() {
        assert_eq!(tg_token(&empty_settings()), None);
    }

    #[test]
    fn test_tg_token_settings_only() {
        let s = settings_with_tg("s_token", "s_chat");
        assert_eq!(tg_token(&s), Some("s_token"));
    }

    #[test]
    fn test_tg_token_settings_some_config_none() {
        let s = Settings {
            telegram_token: Some("only_me".into()),
            ..Default::default()
        };
        assert_eq!(tg_token(&s), Some("only_me"));
    }

    // ── tg_chat_id ──────────────────────────────────────────

    #[test]
    fn test_tg_chat_id_both_none() {
        assert_eq!(tg_chat_id(&empty_settings()), None);
    }

    #[test]
    fn test_tg_chat_id_settings_only() {
        let s = settings_with_tg("t", "-123456");
        assert_eq!(tg_chat_id(&s), Some("-123456"));
    }

    // ── mx_homeserver ───────────────────────────────────────

    #[test]
    fn test_mx_homeserver_both_none() {
        assert_eq!(mx_homeserver(&empty_settings()), None);
    }

    #[test]
    fn test_mx_homeserver_settings_only() {
        let s = settings_with_mx("https://homeserver.test", "tok", "!room:test");
        assert_eq!(mx_homeserver(&s), Some("https://homeserver.test"));
    }

    // ── mx_token ────────────────────────────────────────────

    #[test]
    fn test_mx_token_both_none() {
        assert_eq!(mx_token(&empty_settings()), None);
    }

    #[test]
    fn test_mx_token_settings_only() {
        let s = settings_with_mx("hs", "secret_token", "room");
        assert_eq!(mx_token(&s), Some("secret_token"));
    }

    // ── mx_room ─────────────────────────────────────────────

    #[test]
    fn test_mx_room_both_none() {
        assert_eq!(mx_room(&empty_settings()), None);
    }

    #[test]
    fn test_mx_room_settings_only() {
        let s = settings_with_mx("hs", "tok", "!myroom:test");
        assert_eq!(mx_room(&s), Some("!myroom:test"));
    }

    // ── Early return: notify_telegram ───────────────────────

    #[tokio::test]
    async fn test_notify_telegram_returns_early_no_token() {
        notify_telegram(&empty_settings(), "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_telegram_returns_early_no_chat_id() {
        let s = Settings {
            telegram_token: Some("token".into()),
            ..Default::default()
        };
        notify_telegram(&s, "test", "running").await;
    }

    // ── Early return: notify_matrix ─────────────────────────

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_homeserver() {
        notify_matrix(&empty_settings(), "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_token() {
        let s = Settings {
            matrix_homeserver: Some("https://hs.test".into()),
            ..Default::default()
        };
        notify_matrix(&s, "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_room() {
        let s = Settings {
            matrix_homeserver: Some("https://hs.test".into()),
            matrix_token: Some("tok".into()),
            ..Default::default()
        };
        notify_matrix(&s, "test", "running").await;
    }

    // ── notify_all ──────────────────────────────────────────

    #[tokio::test]
    async fn test_notify_all_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all(&settings, "test", "running").await;
    }
}
