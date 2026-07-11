use crate::config::Config;
use crate::state::http_client;

pub async fn notify_telegram(config: &Config, container: &str, status: &str) {
    let (Some(token), Some(chat_id)) = (&config.telegram_token, &config.telegram_chat_id) else {
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

pub async fn notify_matrix(config: &Config, container: &str, status: &str) {
    let (Some(hs), Some(token), Some(room)) = (
        &config.matrix_homeserver,
        &config.matrix_token,
        &config.matrix_room,
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

pub async fn notify_all(config: &Config, container: &str, status: &str) {
    tokio::join!(
        notify_telegram(config, container, status),
        notify_matrix(config, container, status)
    );
}

/// Notifica solo los canales indicados en `channels` (ej: "telegram", "matrix").
/// Si `channels` está vacío, se notifica a todos los canales configurados.
pub async fn notify_selected(config: &Config, container: &str, status: &str, channels: &[String]) {
    if channels.is_empty() {
        notify_all(config, container, status).await;
        return;
    }
    let mut tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();
    for ch in channels {
        let conf = config.clone();
        let c = container.to_string();
        let s = status.to_string();
        match ch.as_str() {
            "telegram" => {
                tasks.push(tokio::spawn(async move {
                    notify_telegram(&conf, &c, &s).await;
                }));
            }
            "matrix" => {
                tasks.push(tokio::spawn(async move {
                    notify_matrix(&conf, &c, &s).await;
                }));
            }
            _ => {}
        }
    }
    futures::future::join_all(tasks).await;
}
