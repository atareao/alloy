use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::short_digest;
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

/// Select the local digest reference used to compare against the current
/// remote digest: prefer persisted last_remote_digest and fallback to image_id.
pub fn select_local_digest_reference(
    last_remote_digest: Option<&str>,
    image_id: Option<&str>,
) -> String {
    let persisted = last_remote_digest.unwrap_or("").trim();
    if !persisted.is_empty() {
        return persisted.to_string();
    }
    image_id.unwrap_or("").trim().to_string()
}

/// Returns true when remote digest differs from the selected local reference.
pub fn digest_changed(remote_digest: &str, local_reference: &str) -> bool {
    if remote_digest.trim().is_empty() || local_reference.trim().is_empty() {
        return false;
    }
    short_digest(remote_digest) != short_digest(local_reference)
}

#[cfg(test)]
mod tests {
    use super::{digest_changed, select_local_digest_reference};

    #[test]
    fn test_select_local_digest_prefers_persisted() {
        let selected = select_local_digest_reference(Some("sha256:remote"), Some("sha256:local"));
        assert_eq!(selected, "sha256:remote");
    }

    #[test]
    fn test_select_local_digest_fallback_to_image_id() {
        let selected = select_local_digest_reference(None, Some("sha256:local"));
        assert_eq!(selected, "sha256:local");
    }

    #[test]
    fn test_digest_changed_true_when_different() {
        let changed = digest_changed("sha256:bbbbbbbbbbbb", "sha256:aaaaaaaaaaaa");
        assert!(changed);
    }

    #[test]
    fn test_digest_changed_false_when_same() {
        let changed = digest_changed("sha256:aaaaaaaaaaaa", "sha256:aaaaaaaaaaaa");
        assert!(!changed);
    }
}
