use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use chrono::Local;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub async fn record_update_entry(
    db_pool: &DbPool,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    container: &str,
    image: &str,
    old_digest: &str,
    new_digest: &str,
    status: &str,
    duration_ms: u64,
) {
    let entry = UpdateHistoryEntry {
        container: container.to_string(),
        image: image.to_string(),
        old_digest: old_digest.to_string(),
        new_digest: new_digest.to_string(),
        timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        status: status.to_string(),
        duration_ms,
    };
    let mut hist = update_history.lock().await;
    hist.push(entry);
    if let Ok(conn) = db_pool.get().await {
        let _ = crate::db::append_update_history(&conn.lock().unwrap(), hist.last().unwrap());
    }
}

#[allow(dead_code)]
pub async fn notify_update_complete(
    notif_tx: &broadcast::Sender<NotifEvent>,
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
) {
    let ts = Local::now().format("%H:%M:%S").to_string();
    let _ = notif_tx.send(NotifEvent {
        container: container.to_string(),
        status: format!("{} ✅", status),
        timestamp: ts,
    });
    notify_all(settings, container, &format!("✅ {}", status)).await;
}

#[allow(dead_code)]
pub async fn set_updating(db_pool: &DbPool, name: &str) {
    if let Ok(conn) = db_pool.get().await {
        let _ = crate::db::set_updating(&conn.lock().unwrap(), name);
    }
}

#[allow(dead_code)]
pub async fn mark_update_done(db_pool: &DbPool, name: &str) {
    if let Ok(conn) = db_pool.get().await {
        let _ = crate::db::clear_updating(&conn.lock().unwrap(), name);
        let _ = crate::db::update_container_has_update(&conn.lock().unwrap(), name, false);
    }
}

#[allow(dead_code)]
pub async fn clear_updating(db_pool: &DbPool, name: &str) {
    if let Ok(conn) = db_pool.get().await {
        let _ = crate::db::clear_updating(&conn.lock().unwrap(), name);
    }
}
