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

pub async fn notify_telegram(config: &Config, settings: &Settings, container: &str, status: &str) {
    let (Some(token), Some(chat_id)) = (tg_token(settings, config), tg_chat_id(settings, config))
    else {
        return;
    };
    let body = serde_json::json!({"chat_id": chat_id, "text": format!("🪐 *Alloy*\n*{}*: {}", container, status), "parse_mode": "Markdown"});
    if let Err(e) = http_client()
        .post(format!("https://api.telegram.org/bot{}/sendMessage", token))
        .json(&body)
        .send()
        .await
    {
        tracing::error!("Telegram: {}", e);
    }
}

pub async fn notify_matrix(config: &Config, settings: &Settings, container: &str, status: &str) {
    let (Some(hs), Some(token), Some(room)) = (
        mx_homeserver(settings, config),
        mx_token(settings, config),
        mx_room(settings, config),
    ) else {
        return;
    };
    let body =
        serde_json::json!({"msgtype": "m.notice", "body": format!("🐳 {}: {}", container, status)});
    let url = format!(
        "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
        hs.trim_end_matches('/'),
        room,
        uuid::Uuid::new_v4()
    );
    if let Err(e) = http_client()
        .put(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        tracing::error!("Matrix: {}", e);
    }
}

pub async fn notify_all(
    config: &Config,
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
) {
    let s = settings.lock().await;
    tokio::join!(
        notify_telegram(config, &s, container, status),
        notify_matrix(config, &s, container, status)
    );
}

/// Notifica solo los canales indicados en `channels` (ej: "telegram", "matrix").
/// Si `channels` está vacío, se notifica a todos los canales configurados.
pub async fn notify_selected(
    config: &Config,
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
    channels: &[String],
) {
    if channels.is_empty() {
        notify_all(config, settings, container, status).await;
        return;
    }
    let s = settings.lock().await;
    let mut tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    for ch in channels {
        let conf = config.clone();
        let sett = s.clone();
        let c = container.to_string();
        let st = status.to_string();
        match ch.as_str() {
            "telegram" => {
                tasks.push(tokio::spawn(async move {
                    notify_telegram(&conf, &sett, &c, &st).await;
                }));
            }
            "matrix" => {
                tasks.push(tokio::spawn(async move {
                    notify_matrix(&conf, &sett, &c, &st).await;
                }));
            }
            _ => {}
        }
    }
    futures::future::join_all(tasks).await;
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

    // ── notify_selected ─────────────────────────────────────

    #[tokio::test]
    async fn test_notify_selected_empty_channels_does_not_panic() {
        // Empty channels = notify_all = tries both (with no config, both return early)
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_selected(&empty_config(), &settings, "test", "running", &[]).await;
    }

    #[tokio::test]
    async fn test_notify_selected_telegram_channel_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_selected(
            &empty_config(),
            &settings,
            "test",
            "running",
            &["telegram".to_string()],
        )
        .await;
    }

    #[tokio::test]
    async fn test_notify_selected_matrix_channel_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_selected(
            &empty_config(),
            &settings,
            "test",
            "running",
            &["matrix".to_string()],
        )
        .await;
    }

    #[tokio::test]
    async fn test_notify_selected_both_channels_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_selected(
            &empty_config(),
            &settings,
            "test",
            "running",
            &["telegram".to_string(), "matrix".to_string()],
        )
        .await;
    }

    #[tokio::test]
    async fn test_notify_selected_unknown_channel_ignored() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_selected(
            &empty_config(),
            &settings,
            "test",
            "running",
            &["unknown".to_string()],
        )
        .await;
    }

    // ── notify_all ──────────────────────────────────────────

    #[tokio::test]
    async fn test_notify_all_does_not_panic() {
        let settings = Arc::new(Mutex::new(empty_settings()));
        notify_all(&empty_config(), &settings, "test", "running").await;
    }
}
