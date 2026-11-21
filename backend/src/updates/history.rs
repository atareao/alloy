use axum::{extract::State, response::Json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::db::DbPool;
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
    State(db_pool): State<DbPool>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Limpia la memoria dentro de la sección crítica y suelta el guard antes de
    // esperar la pool: evita el ciclo lock(update_history) → pool.
    {
        let mut hist = update_history.lock().await;
        hist.clear();
    }
    let obj = db_pool.get().await.map_err(AppError::from_pool)?;
    let conn = obj.lock().map_err(AppError::from_pool)?;
    if let Err(e) = db::clear_update_history(&conn) {
        tracing::warn!("delete_history_h: no se pudo persistir el borrado del historial: {e}");
    }
    Ok(Json(serde_json::json!({"status": "cleared"})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_pool;

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
                attempts: 0,
            },
            UpdateHistoryEntry {
                container: "new".into(),
                image: "redis".into(),
                old_digest: "c".into(),
                new_digest: "d".into(),
                timestamp: "2024-06-01T00:00:00".into(),
                status: "success".into(),
                duration_ms: 200,
                attempts: 0,
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
                    attempts: 0,
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
                attempts: 0,
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
                attempts: 0,
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
                attempts: 0,
            }]));
        let pool = test_pool();
        let _ = delete_history_h(State(hist.clone()), State(pool)).await;
        let stored = hist.lock().await;
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn test_delete_history_on_empty() {
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let pool = test_pool();
        let result: Json<serde_json::Value> = delete_history_h(State(hist.clone()), State(pool))
            .await
            .unwrap();
        assert_eq!(result.0["status"], "cleared");
        assert!(hist.lock().await.is_empty());
    }

    /// `delete_history_h` no debe retener el guard de `update_history` mientras
    /// espera una conexión de la pool (orden estado → drop → pool).
    #[tokio::test]
    async fn test_delete_history_releases_lock_while_pool_saturated() {
        let pool = crate::db::test_pool_with_max_size(1);
        let _held = pool
            .get()
            .await
            .expect("conexión única para saturar la pool");
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![UpdateHistoryEntry {
                container: "web".into(),
                image: "nginx".into(),
                old_digest: "a".into(),
                new_digest: "b".into(),
                timestamp: "now".into(),
                status: "success".into(),
                duration_ms: 100,
                attempts: 0,
            }]));
        let handler_hist = hist.clone();
        crate::test_support::assert_update_history_lock_free_while_db_pool_saturated(
            hist.clone(),
            async move {
                let _ = delete_history_h(State(handler_hist), State(pool)).await;
            },
        )
        .await;
        // El handler ya limpió la memoria antes de bloquearse en la pool; tras el
        // abort del helper el estado en memoria debe seguir vacío.
        assert!(
            hist.lock().await.is_empty(),
            "el historial en memoria debe quedar vacío"
        );
    }
}
