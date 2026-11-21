use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::Settings;
use crate::state::http_client;
use crate::updates::health_guard::{STATUS_FAILED_STOPPED, STATUS_ROLLED_BACK};

/// Set de codificación para el segmento de ruta del room de Matrix: parte de
/// `NON_ALPHANUMERIC` pero deja los caracteres *unreserved* de RFC 3986
/// (`-._~`) sin codificar; `!` y `:` sí se codifican.
const MATRIX_ROOM_PATH: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Resolve telegram token from settings
fn tg_token(settings: &Settings) -> Option<&str> {
    settings.telegram_token.as_deref()
}

fn tg_chat_id(settings: &Settings) -> Option<&str> {
    settings.telegram_chat_id.as_deref()
}

/// Resolve the Telegram forum topic (message thread) from settings.
///
/// Absent, empty or blank ⇒ `Ok(None)` (notifications go to the general topic).
/// A numeric value (surrounding whitespace ignored) ⇒ `Ok(Some(id))`.
/// Anything else is a hard error: posting to *General* by accident would be
/// silently wrong, so the caller fails fast instead.
fn tg_topic_id(settings: &Settings) -> Result<Option<i64>, String> {
    match settings.telegram_topic_id.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(raw) => raw
            .parse::<i64>()
            .map(Some)
            .map_err(|_| format!("Telegram: topic inválido '{raw}': debe ser un número entero")),
    }
}

/// Build the `sendMessage` body. Without a topic the body is byte-for-byte the
/// legacy one; with a topic `message_thread_id` is added as a **number** (the
/// Telegram API rejects a string there).
fn telegram_payload(chat_id: &str, text: String, topic: Option<i64>) -> serde_json::Value {
    let mut body = serde_json::json!({"chat_id": chat_id, "text": text, "parse_mode": "Markdown"});
    if let Some(thread_id) = topic {
        body["message_thread_id"] = serde_json::json!(thread_id);
    }
    body
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
    let topic = match tg_topic_id(settings) {
        Ok(topic) => topic,
        Err(err) => {
            tracing::error!("{}", err);
            return Err(err);
        }
    };
    let text = format!("🪐 *Alloy*\n*{}*: {}", container, status);
    let body = telegram_payload(chat_id, text, topic);
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
    let room_id = percent_encoding::utf8_percent_encode(room, MATRIX_ROOM_PATH);
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
        hs.trim_end_matches('/'),
        room_id,
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
    notify_all_state(settings, container, status, None).await;
}

/// Fan-out a container-state notification across all channels, deriving the
/// Valet `level` from the **raw Docker state** (`raw_state`) when available.
///
/// `notify_all` delegates here with `raw_state = None` (⇒ `info`), which is the
/// correct semantics for message-based notices (updates, stacks, scheduler).
pub async fn notify_all_state(
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
    raw_state: Option<&str>,
) {
    let level = valet_level_for(raw_state);
    fan_out(settings, container, status, level).await;
}

/// Fan-out a notification with an **explicit** Valet `level`.
///
/// Used by the update guard to force `warning` on a rollback and `critical`
/// (error) on a terminal failure, without relying on the state heuristics of
/// [`valet_level`].
pub async fn notify_all_level(
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
    level: &str,
) {
    fan_out(settings, container, status, level).await;
}

/// Shared fan-out: clone settings, release the lock, then notify every channel.
async fn fan_out(settings: &Arc<Mutex<Settings>>, container: &str, status: &str, level: &str) {
    // Clone the settings and release the `Mutex` guard at the end of this
    // statement, *before* the network fan-out below. Otherwise a slow channel
    // (up to the 30 s HTTP client timeout) would keep the lock and block
    // `GET/PUT /api/config` and the test endpoint.
    let s = settings.lock().await.clone();
    let _ = tokio::join!(
        notify_telegram(&s, container, status),
        notify_matrix(&s, container, status),
        notify_webhook(&s, container, status),
        notify_valet(&s, container, status, level)
    );
}

/// Map a container status to a Valet notification level (deterministic, pure).
///
/// Explicit guard states take priority so they keep their meaning regardless of
/// the substring heuristic below:
/// - `failed-stopped` → `critical` (terminal failure).
/// - `rolled-back` → `warning` (update reverted).
///
/// Otherwise:
/// - `critical` when the status contains `died`, `dead` or `error`.
/// - `warning` when it contains `exit` (exited) or `stop` (stopped).
/// - `info` otherwise.
pub fn valet_level(status: &str) -> &'static str {
    let s = status.to_lowercase();
    if s.contains(STATUS_FAILED_STOPPED) {
        "critical"
    } else if s.contains(STATUS_ROLLED_BACK) {
        "warning"
    } else if s.contains("died") || s.contains("dead") || s.contains("error") {
        "critical"
    } else if s.contains("exit") || s.contains("stop") {
        "warning"
    } else {
        "info"
    }
}

/// Derive the Valet `level` from the raw Docker state, if any.
///
/// `None` (message-based notices) maps to `info`; `Some(state)` delegates to
/// [`valet_level`] so the level is computed over the raw state
/// (`running`/`exited`/`dead`/…) rather than the localized status message.
pub fn valet_level_for(raw_state: Option<&str>) -> &'static str {
    match raw_state {
        Some(state) => valet_level(state),
        None => "info",
    }
}

/// Build the Valet ingest endpoint from a base URL, trimming surrounding
/// whitespace and trailing slashes.
pub fn valet_endpoint(base: &str) -> String {
    format!("{}/api/notifications", base.trim().trim_end_matches('/'))
}

/// Returns `Ok(())` on success, `Err(message)` on failure.
pub async fn notify_valet(
    settings: &Settings,
    container: &str,
    status: &str,
    level: &str,
) -> Result<(), String> {
    let (Some(base), Some(token)) = (
        settings.valet_url.as_deref(),
        settings.valet_token.as_deref(),
    ) else {
        return Err("Valet no configurado: falta URL o token".into());
    };
    let body = serde_json::json!({
        "message": format!("🪐 Alloy: {} → {}", container, status),
        "origin": "alloy",
        "author": "alloy",
        "level": level,
        "metadata": {
            "container": container,
            "status": status,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }
    });
    match http_client()
        .post(valet_endpoint(base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!("Valet: ✅ notificación enviada");
                Ok(())
            } else {
                let s = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let err = format!("Valet: HTTP {} - {}", s, text);
                tracing::error!("{}", err);
                Err(err)
            }
        }
        Err(e) => {
            let err = format!("Valet: error de conexión: {}", e);
            tracing::error!("{}", err);
            Err(err)
        }
    }
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
        "valet" => {
            let msg = "🧪 Test de notificación desde Alloy — Valet funciona correctamente ✅";
            match notify_valet(&s, "Alloy Test", msg, "info").await {
                Ok(()) => Ok(Json(serde_json::json!({
                    "status": "ok",
                    "channel": "valet",
                    "message": "enviado"
                }))),
                Err(e) => Err(crate::models::AppError::Internal(e)),
            }
        }
        other => Err(crate::models::AppError::BadRequest(format!(
            "Canal desconocido: '{}'. Usa 'telegram', 'matrix' o 'valet'.",
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

    /// Telegram channel plus a configured topic (change: telegram-topic-id).
    fn settings_with_tg_topic(topic: &str) -> Settings {
        Settings {
            telegram_token: Some("token".into()),
            telegram_chat_id: Some("chat".into()),
            telegram_topic_id: Some(topic.into()),
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
        let _ = notify_telegram(&empty_settings(), "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_telegram_returns_early_no_chat_id() {
        let s = Settings {
            telegram_token: Some("token".into()),
            ..Default::default()
        };
        let _ = notify_telegram(&s, "test", "running").await;
    }

    // ── Early return: notify_matrix ─────────────────────────

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_homeserver() {
        let _ = notify_matrix(&empty_settings(), "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_token() {
        let s = Settings {
            matrix_homeserver: Some("https://hs.test".into()),
            ..Default::default()
        };
        let _ = notify_matrix(&s, "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_matrix_returns_early_no_room() {
        let s = Settings {
            matrix_homeserver: Some("https://hs.test".into()),
            matrix_token: Some("tok".into()),
            ..Default::default()
        };
        let _ = notify_matrix(&s, "test", "running").await;
    }

    // ── valet_level ─────────────────────────────────────────

    #[test]
    fn test_valet_level_critical() {
        for status in ["died", "dead", "error", "Error", "container died"] {
            assert_eq!(valet_level(status), "critical", "status={status}");
        }
    }

    #[test]
    fn test_valet_level_warning() {
        for status in ["exit", "exited", "stop", "stopped"] {
            assert_eq!(valet_level(status), "warning", "status={status}");
        }
    }

    #[test]
    fn test_valet_level_info() {
        for status in [
            "running",
            "start",
            "started",
            "pause",
            "paused",
            "unpause",
            "restart",
            "restarting",
            "create",
            "created",
        ] {
            assert_eq!(valet_level(status), "info", "status={status}");
        }
    }

    #[test]
    fn test_valet_level_mixed_case_substring() {
        assert_eq!(valet_level("Death"), "info"); // "death" is not one of the keywords
        assert_eq!(valet_level("DIED"), "critical");
        assert_eq!(valet_level("Exited"), "warning");
    }

    // ── valet_level: estados de la guarda de update ──────────

    /// `failed-stopped` es un fallo terminal ⇒ `critical` (gana a la heurística
    /// de `stop` ⇒ `warning`).
    #[test]
    fn test_valet_level_failed_stopped_is_critical() {
        assert_eq!(valet_level("failed-stopped"), "critical");
        assert_eq!(valet_level("FAILED-STOPPED"), "critical");
    }

    /// `rolled-back` es un rollback con aviso ⇒ `warning` (antes caía en `info`).
    #[test]
    fn test_valet_level_rolled_back_is_warning() {
        assert_eq!(valet_level("rolled-back"), "warning");
        assert_eq!(valet_level("Rolled-Back"), "warning");
    }

    // ── valet_level_for (task 2b.2) ─────────────────────────

    #[test]
    fn test_valet_level_for_none_is_info() {
        assert_eq!(valet_level_for(None), "info");
    }

    #[test]
    fn test_valet_level_for_raw_exited_is_warning() {
        assert_eq!(valet_level_for(Some("exited")), "warning");
    }

    #[test]
    fn test_valet_level_for_raw_dead_is_critical() {
        assert_eq!(valet_level_for(Some("dead")), "critical");
    }

    #[test]
    fn test_valet_level_for_raw_running_is_info() {
        assert_eq!(valet_level_for(Some("running")), "info");
    }

    /// The localized status message ("⏹️ detenido") would map to `info` if fed
    /// directly to `valet_level`; the raw Docker state must win instead.
    #[test]
    fn test_valet_level_for_uses_raw_not_localized_message() {
        assert_eq!(valet_level("⏹️ detenido"), "info");
        assert_eq!(valet_level_for(Some("exited")), "warning");
        assert_eq!(valet_level_for(Some("dead")), "critical");
        assert_eq!(valet_level_for(Some("running")), "info");
    }

    // ── valet_endpoint ──────────────────────────────────────

    #[test]
    fn test_valet_endpoint_plain() {
        assert_eq!(
            valet_endpoint("https://v.test"),
            "https://v.test/api/notifications"
        );
    }

    #[test]
    fn test_valet_endpoint_trailing_slash() {
        assert_eq!(
            valet_endpoint("https://v.test/"),
            "https://v.test/api/notifications"
        );
    }

    #[test]
    fn test_valet_endpoint_multiple_trailing_slashes() {
        assert_eq!(
            valet_endpoint("https://v.test///"),
            "https://v.test/api/notifications"
        );
    }

    #[test]
    fn test_valet_endpoint_whitespace_and_trailing_slash() {
        assert_eq!(
            valet_endpoint("  https://v.test/  "),
            "https://v.test/api/notifications"
        );
    }

    // ── Early return: notify_valet ──────────────────────────

    #[tokio::test]
    async fn test_notify_valet_returns_early_no_url() {
        let s = Settings {
            valet_token: Some("val_x".into()),
            ..Default::default()
        };
        let err = notify_valet(&s, "test", "running", "info")
            .await
            .unwrap_err();
        assert!(err.contains("Valet no configurado"), "err={err}");
    }

    #[tokio::test]
    async fn test_notify_valet_returns_early_no_token() {
        let s = Settings {
            valet_url: Some("https://v.test".into()),
            ..Default::default()
        };
        let err = notify_valet(&s, "test", "running", "info")
            .await
            .unwrap_err();
        assert!(err.contains("Valet no configurado"), "err={err}");
    }

    #[tokio::test]
    async fn test_notify_valet_returns_early_both_missing() {
        let err = notify_valet(&empty_settings(), "test", "running", "info")
            .await
            .unwrap_err();
        assert!(err.contains("Valet no configurado"), "err={err}");
    }

    // ── notify_all ──────────────────────────────────────────

    #[tokio::test]
    async fn test_notify_all_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all(&settings, "test", "running").await;
    }

    #[tokio::test]
    async fn test_notify_all_does_not_panic_with_valet_unset() {
        // Valet sin configurar: el fan-out no debe entrar en pánico.
        let settings = Arc::new(Mutex::new(Settings {
            telegram_token: Some("t".into()),
            telegram_chat_id: Some("c".into()),
            ..Default::default()
        }));
        notify_all(&settings, "test", "running").await;
    }

    // ── notify_all_state (task 2b.2) ────────────────────────

    #[tokio::test]
    async fn test_notify_all_state_does_not_panic_with_raw_exited() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all_state(&settings, "test", "⏹️ detenido", Some("exited")).await;
    }

    #[tokio::test]
    async fn test_notify_all_state_does_not_panic_with_raw_dead() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all_state(&settings, "test", "💀 finalizado", Some("dead")).await;
    }

    #[tokio::test]
    async fn test_notify_all_state_does_not_panic_without_raw() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all_state(&settings, "test", "✅ actualizado", None).await;
    }

    // ── lock released during fan-out (change: notify-fanout-lock) ───

    /// The `Mutex<Settings>` must be released before the network fan-out, so a
    /// concurrent `GET/PUT /api/config` (modelled here by `try_lock`) does not
    /// block while a channel is in flight.
    ///
    /// Strategy: a TCP listener accepts one connection and stays hanging, so
    /// the webhook channel stays "in flight" (until its 30 s client timeout).
    /// We spawn the fan-out, wait for the request to be in flight, and assert
    /// the lock is free. The fan-out task is aborted (the 30 s timeout is way
    /// too long for a test).
    #[tokio::test]
    async fn test_notify_all_state_releases_lock_during_fanout() {
        use std::time::Duration;
        use tokio::io::AsyncReadExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let addr = listener.local_addr().expect("local_addr");

        // Server: accept once, read the request head, then hang forever so the
        // webhook stays in flight (no response is ever written). `accepted`
        // signals the test deterministically once the connection is up (no fixed
        // sleep), and the socket is kept alive across the `pending()` await so
        // the client does not see an immediate EOF.
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let accepted2 = Arc::clone(&accepted);
        let notify = Arc::new(tokio::sync::Notify::new());
        let notify2 = Arc::clone(&notify);
        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                accepted2.store(true, std::sync::atomic::Ordering::SeqCst);
                notify2.notify_one();
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Keep the connection open (and `socket` alive) indefinitely.
                std::future::pending::<()>().await;
            }
        });

        let settings = Arc::new(Mutex::new(Settings {
            webhook_url: Some(format!("http://{addr}/hook")),
            ..Default::default()
        }));

        let settings_task = Arc::clone(&settings);
        let fanout = tokio::spawn(async move {
            notify_all_state(&settings_task, "c", "running", None).await;
        });

        // Wait deterministically until the webhook connection is accepted
        // (bounded so a broken test cannot hang forever), then assert the lock
        // is free while the channel is in flight.
        tokio::time::timeout(Duration::from_secs(5), notify.notified())
            .await
            .expect("el servidor de prueba no aceptó la conexión a tiempo");

        assert!(
            accepted.load(std::sync::atomic::Ordering::SeqCst),
            "el servidor de prueba no aceptó la conexión: test no determinista"
        );

        assert!(
            settings.try_lock().is_ok(),
            "el lock de Settings NO se libera durante el fan-out (webhook en vuelo)"
        );

        fanout.abort();
        let _ = fanout.await;
        server.abort();
        let _ = server.await;
    }

    // ── telegram_topic_id (change: telegram-topic-id) ───────

    // GREEN contract pinned by these tests:
    //   fn tg_topic_id(settings: &Settings) -> Result<Option<i64>, String>
    //   fn telegram_payload(chat_id: &str, text: String, topic: Option<i64>) -> serde_json::Value

    // ── tg_topic_id ─────────────────────────────────────────

    /// No topic configured ⇒ `Ok(None)`: the `message_thread_id` key is omitted.
    #[test]
    fn test_tg_topic_id_absent_is_ok_none() {
        assert_eq!(tg_topic_id(&settings_with_tg("tok", "chat")), Ok(None));
    }

    #[test]
    fn test_tg_topic_id_valid_is_ok_some() {
        assert_eq!(tg_topic_id(&settings_with_tg_topic("42")), Ok(Some(42)));
    }

    /// The value is `trim`med before parsing, so stray spaces are still valid.
    #[test]
    fn test_tg_topic_id_whitespace_wrapped_is_ok_some() {
        assert_eq!(tg_topic_id(&settings_with_tg_topic("  42  ")), Ok(Some(42)));
    }

    /// Scenario: Topic inválido (D3). A non-numeric topic is a hard error, so
    /// `notify_telegram` fails fast instead of silently posting to *General*.
    #[test]
    fn test_tg_topic_id_non_numeric_is_err() {
        let err = tg_topic_id(&settings_with_tg_topic("abc")).unwrap_err();
        assert!(
            err.to_lowercase().contains("topic"),
            "el error debe identificar el topic inválido: {err}"
        );
    }

    // ── telegram_payload ────────────────────────────────────

    /// Scenario: Sin topic configurado — the body must be byte-for-byte the
    /// legacy body (no `message_thread_id` key at all).
    #[test]
    fn test_telegram_payload_without_topic_is_legacy_body() {
        let body = telegram_payload("-100123", "hola".into(), None);
        assert_eq!(
            body,
            serde_json::json!({
                "chat_id": "-100123",
                "text": "hola",
                "parse_mode": "Markdown",
            })
        );
        assert!(
            body.get("message_thread_id").is_none(),
            "sin topic la clave message_thread_id no debe aparecer: {body}"
        );
    }

    /// Scenario: Envío a un topic concreto — `message_thread_id` must be a
    /// **number** (the Telegram API rejects a string here).
    #[test]
    fn test_telegram_payload_with_topic_has_numeric_thread_id() {
        let body = telegram_payload("-100123", "hola".into(), Some(42));
        assert_eq!(body["message_thread_id"], serde_json::json!(42));
        assert!(
            body["message_thread_id"].is_i64(),
            "message_thread_id debe ser numérico, no string: {body}"
        );
        // The rest of the body is untouched.
        assert_eq!(body["chat_id"], "-100123");
        assert_eq!(body["text"], "hola");
        assert_eq!(body["parse_mode"], "Markdown");
    }

    // ── notify_telegram con topic inválido ──────────────────

    /// Scenario: Topic inválido — `notify_telegram` returns `Err` **without**
    /// emitting the HTTP request. The error is the topic error (a failed HTTP
    /// round-trip would instead surface as a transport error), which is what
    /// makes this assertion meaningful without a network mock.
    #[tokio::test]
    async fn test_notify_telegram_invalid_topic_returns_err_without_http() {
        let s = settings_with_tg_topic("abc");
        let err = notify_telegram(&s, "test", "running").await.unwrap_err();
        assert!(
            err.to_lowercase().contains("topic"),
            "topic inválido ⇒ Err antes de emitir HTTP (no un error de red): {err}"
        );
    }

    // ── notify_matrix: endpoint v3 + room encoded (change: matrix-v3-endpoint) ──

    /// Servidor HTTP local de un solo uso: captura la petición (cabeceras +
    /// cuerpo según `Content-Length`) y responde con `status_line`/`body`.
    async fn spawn_capture_server_with(
        status_line: &'static str,
        body: &'static str,
    ) -> (u16, tokio::sync::oneshot::Receiver<String>) {
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let port = listener.local_addr().expect("local_addr").port();
        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf: Vec<u8> = Vec::new();
                let mut tmp = [0u8; 4096];
                let header_end = loop {
                    match tokio::time::timeout(Duration::from_millis(200), socket.read(&mut tmp))
                        .await
                    {
                        Ok(Ok(0)) | Ok(Err(_)) | Err(_) => break None,
                        Ok(Ok(n)) => {
                            buf.extend_from_slice(&tmp[..n]);
                            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                                break Some(pos + 4);
                            }
                        }
                    }
                };
                if let Some(header_end) = header_end {
                    let content_length = String::from_utf8_lossy(&buf[..header_end])
                        .lines()
                        .find_map(|line| {
                            let (k, v) = line.split_once(':')?;
                            if k.eq_ignore_ascii_case("content-length") {
                                v.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                    while buf.len() < header_end + content_length {
                        match tokio::time::timeout(
                            Duration::from_millis(200),
                            socket.read(&mut tmp),
                        )
                        .await
                        {
                            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => break,
                            Ok(Ok(n)) => buf.extend_from_slice(&tmp[..n]),
                        }
                    }
                }
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
            }
        });

        (port, rx)
    }

    /// Variante de éxito (`200 OK`, cuerpo `{}`).
    async fn spawn_capture_server() -> (u16, tokio::sync::oneshot::Receiver<String>) {
        spawn_capture_server_with("200 OK", "{}").await
    }

    /// Escenario (spec `notifications`): envío correcto con el endpoint v3.
    #[tokio::test]
    async fn test_notify_matrix_uses_v3_and_encodes_room() {
        let (port, rx) = spawn_capture_server().await;
        let s = Settings {
            matrix_homeserver: Some(format!("http://127.0.0.1:{port}/")),
            matrix_token: Some("tok".into()),
            matrix_room: Some("!abc:matrix.org".into()),
            ..Default::default()
        };

        let res = notify_matrix(&s, "app", "running").await;
        assert!(res.is_ok(), "notify_matrix debió ir bien: {res:?}");

        let req = rx.await.expect("captura de petición");
        let request_line = req.lines().next().unwrap_or_default();
        assert!(
            request_line.starts_with("PUT "),
            "método != PUT: {request_line}"
        );
        assert!(
            request_line
                .contains("/_matrix/client/v3/rooms/%21abc%3Amatrix.org/send/m.room.message/"),
            "URL sin v3 ni room codificado: {request_line}"
        );
        assert!(!req.contains("/r0/"), "sigue usando r0: {request_line}");
        assert!(
            req.to_ascii_lowercase()
                .contains("authorization: bearer tok"),
            "falta el Bearer: {req}"
        );
        assert!(
            req.to_ascii_lowercase()
                .contains("content-type: application/json"),
            "falta Content-Type: {req}"
        );
        assert!(req.contains("m.notice"), "cuerpo sin m.notice: {req}");
        assert!(
            req.contains("🐳 app: running"),
            "cuerpo sin el texto esperado: {req}"
        );
    }

    /// Escenario (spec `notifications`): el ID de sala va codificado y sin `r0`.
    #[tokio::test]
    async fn test_notify_matrix_does_not_use_r0() {
        let (port, rx) = spawn_capture_server().await;
        let s = Settings {
            matrix_homeserver: Some(format!("http://127.0.0.1:{port}")),
            matrix_token: Some("tok".into()),
            matrix_room: Some("!abc:matrix.org".into()),
            ..Default::default()
        };

        let _ = notify_matrix(&s, "app", "running").await;

        let req = rx.await.expect("captura de petición");
        let request_line = req.lines().next().unwrap_or_default();
        assert!(
            !request_line.contains("/r0/"),
            "no debe usar r0: {request_line}"
        );
        assert!(
            !request_line.contains("!abc:matrix.org"),
            "room sin codificar: {request_line}"
        );
    }

    /// Escenario (spec): homeserver con barras finales no produce dobles barras.
    #[tokio::test]
    async fn test_notify_matrix_trims_trailing_slash() {
        let (port, rx) = spawn_capture_server().await;
        let s = Settings {
            matrix_homeserver: Some(format!("http://127.0.0.1:{port}/")),
            matrix_token: Some("tok".into()),
            matrix_room: Some("!room:hs.test".into()),
            ..Default::default()
        };

        assert!(notify_matrix(&s, "app", "running").await.is_ok());

        let req = rx.await.expect("captura de petición");
        let request_line = req.lines().next().unwrap_or_default();
        assert!(
            request_line.starts_with("PUT /_matrix/client/v3/"),
            "URL con doble barra o prefijo incorrecto: {request_line}"
        );
    }

    /// Escenario (spec): `txn_id` único por envío.
    #[tokio::test]
    async fn test_notify_matrix_txn_id_is_unique_per_send() {
        let s = |port: u16| Settings {
            matrix_homeserver: Some(format!("http://127.0.0.1:{port}")),
            matrix_token: Some("tok".into()),
            matrix_room: Some("!room:hs.test".into()),
            ..Default::default()
        };

        let (p1, rx1) = spawn_capture_server().await;
        assert!(notify_matrix(&s(p1), "app", "running").await.is_ok());
        let req1 = rx1.await.expect("captura 1");

        let (p2, rx2) = spawn_capture_server().await;
        assert!(notify_matrix(&s(p2), "app", "running").await.is_ok());
        let req2 = rx2.await.expect("captura 2");

        let txn = |req: &str| {
            req.lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_string()
        };
        assert_ne!(txn(&req1), txn(&req2), "txn_id repetido entre envíos");
    }

    /// Escenario (spec): Matrix rechaza con HTTP 500.
    #[tokio::test]
    async fn test_notify_matrix_http_500_returns_err_with_body() {
        let (port, _rx) = spawn_capture_server_with(
            "500 Internal Server Error",
            r#"{"errcode":"M_UNKNOWN","error":"boom"}"#,
        )
        .await;
        let s = Settings {
            matrix_homeserver: Some(format!("http://127.0.0.1:{port}")),
            matrix_token: Some("tok".into()),
            matrix_room: Some("!room:hs.test".into()),
            ..Default::default()
        };

        let err = notify_matrix(&s, "app", "running")
            .await
            .expect_err("debe fallar con 500");
        assert!(err.contains("HTTP 500"), "sin código HTTP: {err}");
        assert!(err.contains("M_UNKNOWN"), "sin cuerpo de respuesta: {err}");
    }

    /// Escenario (spec): error de conexión con Matrix.
    #[tokio::test]
    async fn test_notify_matrix_connection_error_returns_err() {
        let port = {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            l.local_addr().expect("addr").port()
        };
        let s = Settings {
            matrix_homeserver: Some(format!("http://127.0.0.1:{port}")),
            matrix_token: Some("tok".into()),
            matrix_room: Some("!room:hs.test".into()),
            ..Default::default()
        };

        let err = notify_matrix(&s, "app", "running")
            .await
            .expect_err("debe fallar la conexión");
        assert!(
            err.contains("error de conexión"),
            "mensaje inesperado: {err}"
        );
    }
}
