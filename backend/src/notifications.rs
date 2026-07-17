use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::models::Settings;
use crate::state::http_client;

/// Resolve telegram token: settings overlay config
fn tg_token<'a>(settings: &'a Settings, config: &'a Config) -> Option<&'a str> {
    settings
        .telegram_token
        .as_deref()
        .or(config.telegram_token.as_deref())
}

fn tg_chat_id<'a>(settings: &'a Settings, config: &'a Config) -> Option<&'a str> {
    settings
        .telegram_chat_id
        .as_deref()
        .or(config.telegram_chat_id.as_deref())
}

fn mx_homeserver<'a>(settings: &'a Settings, config: &'a Config) -> Option<&'a str> {
    settings
        .matrix_homeserver
        .as_deref()
        .or(config.matrix_homeserver.as_deref())
}

fn mx_token<'a>(settings: &'a Settings, config: &'a Config) -> Option<&'a str> {
    settings
        .matrix_token
        .as_deref()
        .or(config.matrix_token.as_deref())
}

fn mx_room<'a>(settings: &'a Settings, config: &'a Config) -> Option<&'a str> {
    settings
        .matrix_room
        .as_deref()
        .or(config.matrix_room.as_deref())
}

/// Returns `Ok(())` on success, `Err(message)` on failure.
pub async fn notify_telegram(
    config: &Config,
    settings: &Settings,
    container: &str,
    status: &str,
) -> Result<(), String> {
    let (Some(token), Some(chat_id)) = (tg_token(settings, config), tg_chat_id(settings, config))
    else {
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
    config: &Config,
    settings: &Settings,
    container: &str,
    status: &str,
) -> Result<(), String> {
    let (hs, token, room) = (
        mx_homeserver(settings, config),
        mx_token(settings, config),
        mx_room(settings, config),
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
    config: &Config,
    settings: &Settings,
    container: &str,
    status: &str,
) -> Result<(), String> {
    let url = settings
        .webhook_url
        .as_deref()
        .or(config.webhook_url.as_deref());
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

pub async fn notify_all(
    config: &Config,
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
) {
    let s = settings.lock().await;
    let _ = tokio::join!(
        notify_telegram(config, &s, container, status),
        notify_matrix(config, &s, container, status),
        notify_webhook(config, &s, container, status)
    );
}

// ── Test notification endpoint ──────────────────────────────

use axum::{extract::State, response::Json, routing::post, Router};

use crate::models::TestNotificationReq;
use crate::state::AppState;

async fn test_notification_h(
    State(config): State<crate::config::Config>,
    State(settings): State<std::sync::Arc<tokio::sync::Mutex<crate::models::Settings>>>,
    Json(body): Json<TestNotificationReq>,
) -> Result<Json<serde_json::Value>, crate::models::AppError> {
    let s = settings.lock().await;
    match body.channel.as_str() {
        "telegram" => {
            let msg = "🧪 Test de notificación desde Alloy — Telegram funciona correctamente ✅";
            match notify_telegram(&config, &s, "Alloy Test", msg).await {
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
            match notify_matrix(&config, &s, "Alloy Test", msg).await {
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

    // ── Helpers ─────────────────────────────────────────────
    fn empty_settings() -> Settings {
        Settings::default()
    }

    fn empty_config() -> Config {
        Config::default()
    }

    fn settings_with_tg(token: &str, chat_id: &str) -> Settings {
        Settings {
            telegram_token: Some(token.into()),
            telegram_chat_id: Some(chat_id.into()),
            ..Default::default()
        }
    }

    fn config_with_tg(token: &str, chat_id: &str) -> Config {
        Config {
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

    fn config_with_mx(hs: &str, token: &str, room: &str) -> Config {
        Config {
            matrix_homeserver: Some(hs.into()),
            matrix_token: Some(token.into()),
            matrix_room: Some(room.into()),
            ..Default::default()
        }
    }

    // ── tg_token ────────────────────────────────────────────

    #[test]
    fn test_tg_token_both_none() {
        assert_eq!(tg_token(&empty_settings(), &empty_config()), None);
    }

    #[test]
    fn test_tg_token_settings_only() {
        let s = settings_with_tg("s_token", "s_chat");
        assert_eq!(tg_token(&s, &empty_config()), Some("s_token"));
    }

    #[test]
    fn test_tg_token_config_only() {
        let c = config_with_tg("c_token", "c_chat");
        assert_eq!(tg_token(&empty_settings(), &c), Some("c_token"));
    }

    #[test]
    fn test_tg_token_settings_overrides_config() {
        let s = settings_with_tg("s_token", "s_chat");
        let c = config_with_tg("c_token", "c_chat");
        assert_eq!(tg_token(&s, &c), Some("s_token"));
    }

    #[test]
    fn test_tg_token_settings_some_config_none() {
        let s = Settings {
            telegram_token: Some("only_me".into()),
            ..Default::default()
        };
        assert_eq!(tg_token(&s, &empty_config()), Some("only_me"));
    }

    // ── tg_chat_id ──────────────────────────────────────────

    #[test]
    fn test_tg_chat_id_both_none() {
        assert_eq!(tg_chat_id(&empty_settings(), &empty_config()), None);
    }

    #[test]
    fn test_tg_chat_id_settings_only() {
        let s = settings_with_tg("t", "-123456");
        assert_eq!(tg_chat_id(&s, &empty_config()), Some("-123456"));
    }

    #[test]
    fn test_tg_chat_id_config_only() {
        let c = config_with_tg("t", "-987654");
        assert_eq!(tg_chat_id(&empty_settings(), &c), Some("-987654"));
    }

    #[test]
    fn test_tg_chat_id_settings_overrides_config() {
        let s = settings_with_tg("t", "from_settings");
        let c = config_with_tg("t", "from_config");
        assert_eq!(tg_chat_id(&s, &c), Some("from_settings"));
    }

    // ── mx_homeserver ───────────────────────────────────────

    #[test]
    fn test_mx_homeserver_both_none() {
        assert_eq!(mx_homeserver(&empty_settings(), &empty_config()), None);
    }

    #[test]
    fn test_mx_homeserver_settings_only() {
        let s = settings_with_mx("https://homeserver.test", "tok", "!room:test");
        assert_eq!(
            mx_homeserver(&s, &empty_config()),
            Some("https://homeserver.test")
        );
    }

    #[test]
    fn test_mx_homeserver_config_only() {
        let c = config_with_mx("https://config.test", "tok", "!room:config");
        assert_eq!(
            mx_homeserver(&empty_settings(), &c),
            Some("https://config.test")
        );
    }

    #[test]
    fn test_mx_homeserver_settings_overrides_config() {
        let s = settings_with_mx("https://settings.test", "tok", "!room:s");
        let c = config_with_mx("https://config.test", "tok", "!room:c");
        assert_eq!(mx_homeserver(&s, &c), Some("https://settings.test"));
    }

    // ── mx_token ────────────────────────────────────────────

    #[test]
    fn test_mx_token_both_none() {
        assert_eq!(mx_token(&empty_settings(), &empty_config()), None);
    }

    #[test]
    fn test_mx_token_settings_only() {
        let s = settings_with_mx("hs", "secret_token", "room");
        assert_eq!(mx_token(&s, &empty_config()), Some("secret_token"));
    }

    #[test]
    fn test_mx_token_config_only() {
        let c = config_with_mx("hs", "config_token", "room");
        assert_eq!(mx_token(&empty_settings(), &c), Some("config_token"));
    }

    #[test]
    fn test_mx_token_settings_overrides_config() {
        let s = settings_with_mx("hs", "s_token", "room");
        let c = config_with_mx("hs", "c_token", "room");
        assert_eq!(mx_token(&s, &c), Some("s_token"));
    }

    // ── mx_room ─────────────────────────────────────────────

    #[test]
    fn test_mx_room_both_none() {
        assert_eq!(mx_room(&empty_settings(), &empty_config()), None);
    }

    #[test]
    fn test_mx_room_settings_only() {
        let s = settings_with_mx("hs", "tok", "!myroom:test");
        assert_eq!(mx_room(&s, &empty_config()), Some("!myroom:test"));
    }

    #[test]
    fn test_mx_room_config_only() {
        let c = config_with_mx("hs", "tok", "!config:room");
        assert_eq!(mx_room(&empty_settings(), &c), Some("!config:room"));
    }

    #[test]
    fn test_mx_room_settings_overrides_config() {
        let s = settings_with_mx("hs", "tok", "!settings:room");
        let c = config_with_mx("hs", "tok", "!config:room");
        assert_eq!(mx_room(&s, &c), Some("!settings:room"));
    }

    // ── Early return: notify_telegram ───────────────────────

    #[tokio::test]
    async fn test_notify_telegram_returns_early_no_token() {
        // Should not panic or make HTTP calls when telegram_token is missing
        notify_telegram(&empty_config(), &empty_settings(), "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_telegram_returns_early_no_chat_id() {
        let s = Settings {
            telegram_token: Some("token".into()),
            ..Default::default()
        };
        notify_telegram(&empty_config(), &s, "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_telegram_returns_early_token_in_config_chat_id_missing() {
        let c = config_with_tg("token", "chat_id");
        notify_telegram(&c, &empty_settings(), "test", "running").await;
    }

    // ── Early return: notify_matrix ─────────────────────────

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_homeserver() {
        notify_matrix(&empty_config(), &empty_settings(), "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_token() {
        let s = Settings {
            matrix_homeserver: Some("https://hs.test".into()),
            ..Default::default()
        };
        notify_matrix(&empty_config(), &s, "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_room() {
        let s = Settings {
            matrix_homeserver: Some("https://hs.test".into()),
            matrix_token: Some("tok".into()),
            ..Default::default()
        };
        notify_matrix(&empty_config(), &s, "test", "running").await;
    }

    // ── notify_all ──────────────────────────────────────────

    #[tokio::test]
    async fn test_notify_all_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all(&empty_config(), &settings, "test", "running").await;
    }
}
