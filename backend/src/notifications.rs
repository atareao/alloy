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
