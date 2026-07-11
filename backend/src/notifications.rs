use crate::config::Config;

use crate::state::http_client;

pub async fn notify_telegram(config: &Config, container: &str, status: &str) {
    let (Some(token), Some(chat_id)) = (&config.telegram_token, &config.telegram_chat_id) else {
        return;
    };
    let body = serde_json::json!({"chat_id": chat_id, "text": format!("🐳 *Cabina Docker*\n*{}*: {}", container, status), "parse_mode": "Markdown"});
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
