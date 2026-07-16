use axum::{extract::State, response::Json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::models::*;

pub async fn get_history_h(
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<Vec<UpdateHistoryEntry>> {
    let hist = update_history.lock().await;
    let mut sorted = hist.clone();
    sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    sorted.truncate(100);
    Json(sorted)
}

pub async fn delete_history_h(
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
) -> Json<serde_json::Value> {
    let mut hist = update_history.lock().await;
    hist.clear();
    let conn = db::global().lock().await;
    let _ = db::clear_update_history(&conn);
    Json(serde_json::json!({"status": "cleared"}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_history_empty() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let result: Json<Vec<UpdateHistoryEntry>> = get_history_h(State(hist)).await;
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_get_history_sorted_by_timestamp() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(vec![
            UpdateHistoryEntry {
                container: "old".into(),
                image: "nginx".into(),
                old_digest: "a".into(),
                new_digest: "b".into(),
                timestamp: "2024-01-01T00:00:00".into(),
                status: "success".into(),
                duration_ms: 100,
            },
            UpdateHistoryEntry {
                container: "new".into(),
                image: "redis".into(),
                old_digest: "c".into(),
                new_digest: "d".into(),
                timestamp: "2024-06-01T00:00:00".into(),
                status: "success".into(),
                duration_ms: 200,
            },
        ]));
        let result: Json<Vec<UpdateHistoryEntry>> = get_history_h(State(hist)).await;
        assert_eq!(result.0.len(), 2);
        assert_eq!(result.0[0].container, "new");
        assert_eq!(result.0[1].container, "old");
    }

    #[tokio::test]
    async fn test_get_history_truncated_to_100() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let mut data = hist.lock().await;
            for i in 0..150 {
                data.push(UpdateHistoryEntry {
                    container: format!("c{}", i),
                    image: "img".into(),
                    old_digest: String::new(),
                    new_digest: String::new(),
                    timestamp: format!("2024-{:02}-01T00:00:00", (i % 12) + 1),
                    status: "success".into(),
                    duration_ms: i as u64,
                });
            }
        }
        let result: Json<Vec<UpdateHistoryEntry>> = get_history_h(State(hist)).await;
        assert_eq!(result.0.len(), 100);
    }

    #[tokio::test]
    async fn test_get_history_with_errors() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![UpdateHistoryEntry {
                container: "web".into(),
                image: "nginx".into(),
                old_digest: "old".into(),
                new_digest: "new".into(),
                timestamp: "2024-01-01T00:00:00".into(),
                status: "error".into(),
                duration_ms: 50,
            }]));
        let result: Json<Vec<UpdateHistoryEntry>> = get_history_h(State(hist)).await;
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].status, "error");
    }

    #[tokio::test]
    async fn test_get_history_duration_ms() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![UpdateHistoryEntry {
                container: "web".into(),
                image: "nginx".into(),
                old_digest: "a".into(),
                new_digest: "b".into(),
                timestamp: "2024-01-01T00:00:00".into(),
                status: "success".into(),
                duration_ms: 12345,
            }]));
        let result: Json<Vec<UpdateHistoryEntry>> = get_history_h(State(hist)).await;
        assert_eq!(result.0[0].duration_ms, 12345);
    }

    #[tokio::test]
    async fn test_delete_history_clears() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![UpdateHistoryEntry {
                container: "web".into(),
                image: "nginx".into(),
                old_digest: "a".into(),
                new_digest: "b".into(),
                timestamp: "now".into(),
                status: "success".into(),
                duration_ms: 100,
            }]));
        let _ = delete_history_h(State(hist.clone())).await;
        let stored = hist.lock().await;
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn test_delete_history_on_empty() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> = delete_history_h(State(hist.clone())).await;
        assert_eq!(result.0["status"], "cleared");
        assert!(hist.lock().await.is_empty());
    }
}
