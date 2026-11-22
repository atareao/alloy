use axum::extract::{Query, State};
use axum::response::Json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::db::DbPool;
use crate::models::*;

/// `GET /api/history` — página del histórico persistido, con filtros.
///
/// Acepta `page`, `page_size`, `q` y `status` (ver [`UpdateHistoryQuery`]) y
/// devuelve `{ items, total, page, page_size }`. La página se resuelve contra
/// SQLite (no contra el vector en memoria): `total` refleja el histórico
/// completo —sin el antiguo cap de 100— y el handler no toma
/// `update_history.lock()`, de modo que una pool saturada no bloquea a los
/// workers que publican entradas. `total` e `items` se leen en una única
/// transacción (`db::load_update_history_page_with_total`), así que describen
/// la misma instantánea aunque un worker inserte entre medias.
pub async fn get_history_h(
    State(db_pool): State<DbPool>,
    Query(query): Query<UpdateHistoryQuery>,
) -> Result<Json<PagedUpdateHistory>, AppError> {
    let params = query.normalize()?;

    // Se resuelve la página contra la pool **antes** de cualquier lock en
    // memoria: el handler no toma `update_history.lock()`, de modo que una pool
    // saturada no bloquea a los workers que publican entradas.
    let obj = db_pool.get().await.map_err(AppError::from_pool)?;
    let mut conn = obj.lock().map_err(AppError::from_pool)?;

    let (total, items) =
        db::load_update_history_page_with_total(&mut conn, &params).map_err(AppError::from_db)?;

    Ok(Json(PagedUpdateHistory {
        items,
        total,
        page: params.page,
        page_size: params.page_size,
    }))
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
    use crate::db::test_pool_with_max_size;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    /// Pool con una única conexión: en SQLite `:memory:` cada conexión abre su
    /// propia base de datos, así que el sembrado y la lectura deben compartir la
    /// misma conexión.
    fn pool() -> DbPool {
        test_pool_with_max_size(1)
    }

    fn entry(container: &str, timestamp: &str) -> UpdateHistoryEntry {
        UpdateHistoryEntry {
            container: container.into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: timestamp.into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        }
    }

    /// Persiste `entries` en la pool (misma conexión única de `pool()`).
    async fn seed(pool: &DbPool, entries: &[UpdateHistoryEntry]) {
        let obj = pool.get().await.expect("conexión de test");
        let conn = obj.lock().expect("lock de conexión");
        for e in entries {
            db::append_update_history(&conn, e).expect("append_update_history");
        }
    }

    // ── migrados al contrato paginado ────────────────────────

    #[tokio::test]
    async fn test_get_history_empty() {
        let pool = pool();
        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico vacío");
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 20);
    }

    #[tokio::test]
    async fn test_get_history_sorted_by_timestamp() {
        let pool = pool();
        seed(
            &pool,
            &[
                entry("old", "2024-01-01T00:00:00"),
                entry("new", "2024-06-01T00:00:00"),
            ],
        )
        .await;
        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].container, "new");
        assert_eq!(page.items[1].container, "old");
    }

    /// Antes se truncaba a 100; ahora `total` refleja el histórico real.
    #[tokio::test]
    async fn test_get_history_total_not_capped_at_100() {
        let pool = pool();
        let entries: Vec<UpdateHistoryEntry> = (0..150)
            .map(|i| {
                entry(
                    &format!("c{i}"),
                    &format!("2024-{:02}-01T00:00:00", (i % 12) + 1),
                )
            })
            .collect();
        seed(&pool, &entries).await;
        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.total, 150, "el cap de 100 ya no aplica");
        assert_eq!(
            page.items.len(),
            20,
            "la página por defecto sigue siendo de 20"
        );
    }

    #[tokio::test]
    async fn test_get_history_with_errors() {
        let pool = pool();
        let mut e = entry("web", "2024-01-01T00:00:00");
        e.status = "error".into();
        seed(&pool, &[e]).await;
        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].status, "error");
    }

    #[tokio::test]
    async fn test_get_history_duration_ms() {
        let pool = pool();
        let mut e = entry("web", "2024-01-01T00:00:00");
        e.duration_ms = 12345;
        seed(&pool, &[e]).await;
        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.items[0].duration_ms, 12345);
    }

    #[tokio::test]
    async fn test_delete_history_clears() {
        let pool = pool();
        seed(&pool, &[entry("web", "now")]).await;
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![entry("web", "now")]));
        let _ = delete_history_h(State(hist.clone()), State(pool.clone())).await;
        assert!(hist.lock().await.is_empty(), "la memoria debe quedar vacía");

        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.total, 0);
        assert!(page.items.is_empty());
    }

    #[tokio::test]
    async fn test_delete_history_on_empty() {
        let pool = pool();
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> =
            delete_history_h(State(hist.clone()), State(pool.clone()))
                .await
                .unwrap();
        assert_eq!(result.0["status"], "cleared");
        assert!(hist.lock().await.is_empty());

        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.total, 0);
        assert!(page.items.is_empty());
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
            Arc::new(Mutex::new(vec![entry("web", "now")]));
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

    // ── nuevos: update-history-pagination ────────────────────

    /// 1.2 — una query inválida (`page=0`) se mapea a `400` desde el handler.
    #[tokio::test]
    async fn test_get_history_invalid_query_is_bad_request() {
        let pool = pool();
        let err = get_history_h(
            State(pool),
            Query(UpdateHistoryQuery {
                page: Some(0),
                ..Default::default()
            }),
        )
        .await
        .expect_err("page=0 debe rechazarse con 400");
        assert!(matches!(&err, AppError::BadRequest(_)));
        assert_eq!(
            err.into_response().status(),
            axum::http::StatusCode::BAD_REQUEST
        );
    }

    /// 1.3 — GIVEN 25 entradas WHEN `GET /api/history` THEN 20 items y total 25.
    #[tokio::test]
    async fn test_get_history_default_page_returns_first_twenty_of_twentyfive() {
        let pool = pool();
        let entries: Vec<UpdateHistoryEntry> = (0..25)
            .map(|i| entry(&format!("c{i:02}"), &format!("2024-01-01T00:00:{i:02}")))
            .collect();
        seed(&pool, &entries).await;
        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.page, 1);
        assert_eq!(page.page_size, 20);
        assert_eq!(page.total, 25);
        assert_eq!(page.items.len(), 20);
    }

    /// 1.3 — GIVEN 25 entradas WHEN `page=2&page_size=20` THEN 5 items, total 25.
    #[tokio::test]
    async fn test_get_history_second_page_returns_remaining_five() {
        let pool = pool();
        let entries: Vec<UpdateHistoryEntry> = (0..25)
            .map(|i| entry(&format!("c{i:02}"), &format!("2024-01-01T00:00:{i:02}")))
            .collect();
        seed(&pool, &entries).await;
        let Json(page) = get_history_h(
            State(pool),
            Query(UpdateHistoryQuery {
                page: Some(2),
                page_size: Some(20),
                ..Default::default()
            }),
        )
        .await
        .expect("histórico");
        assert_eq!(page.items.len(), 5);
        assert_eq!(page.total, 25);
    }

    /// 1.3 — GIVEN 25 entradas WHEN `page=99` THEN items vacío y total 25.
    #[tokio::test]
    async fn test_get_history_page_beyond_end_is_empty_and_keeps_total() {
        let pool = pool();
        let entries: Vec<UpdateHistoryEntry> = (0..25)
            .map(|i| entry(&format!("c{i:02}"), &format!("2024-01-01T00:00:{i:02}")))
            .collect();
        seed(&pool, &entries).await;
        let Json(page) = get_history_h(
            State(pool),
            Query(UpdateHistoryQuery {
                page: Some(99),
                ..Default::default()
            }),
        )
        .await
        .expect("histórico");
        assert!(page.items.is_empty());
        assert_eq!(page.total, 25);
    }

    /// 1.5 — `q` filtra por `container` case-insensitive desde el handler.
    #[tokio::test]
    async fn test_get_history_q_filter_is_case_insensitive() {
        let pool = pool();
        seed(
            &pool,
            &[
                entry("web", "2024-01-01T00:00:00"),
                entry("db", "2024-01-02T00:00:00"),
                entry("cache", "2024-01-03T00:00:00"),
            ],
        )
        .await;
        let Json(page) = get_history_h(
            State(pool),
            Query(UpdateHistoryQuery {
                q: Some("WEB".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("histórico");
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].container, "web");
    }

    /// 1.7 — tras el borrado, memoria y persistencia quedan vacías.
    #[tokio::test]
    async fn test_delete_history_clears_memory_and_persistence() {
        let pool = pool();
        seed(&pool, &[entry("web", "now"), entry("db", "now")]).await;
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![entry("web", "now"), entry("db", "now")]));
        let _ = delete_history_h(State(hist.clone()), State(pool.clone())).await;
        assert!(hist.lock().await.is_empty());

        {
            let obj = pool.get().await.expect("conexión de test");
            let conn = obj.lock().expect("lock de conexión");
            assert!(
                db::load_update_history(&conn).unwrap().is_empty(),
                "la persistencia debe quedar vacía"
            );
        }

        let Json(page) = get_history_h(State(pool), Query(UpdateHistoryQuery::default()))
            .await
            .expect("histórico");
        assert_eq!(page.total, 0);
        assert!(page.items.is_empty());
    }

    /// 1.8 — la lectura no retiene el lock del histórico con la pool saturada.
    #[tokio::test]
    async fn test_get_history_releases_lock_while_pool_saturated() {
        let pool = crate::db::test_pool_with_max_size(1);
        let _held = pool
            .get()
            .await
            .expect("conexión única para saturar la pool");
        let hist: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![entry("web", "now")]));
        crate::test_support::assert_update_history_lock_free_while_db_pool_saturated(
            hist,
            async move {
                let _ = get_history_h(State(pool), Query(UpdateHistoryQuery::default())).await;
            },
        )
        .await;
    }

    // ── E2E a nivel de router/HTTP ───────────────────────────

    /// Monta el router real de `updates` sobre un `AppState` de test.
    ///
    /// A diferencia de los tests de handler, aquí se ejercita el borde HTTP
    /// completo (extractores, mapeo de errores y serialización del envelope).
    fn http_app(pool: DbPool) -> axum::Router {
        crate::updates::routes().with_state(crate::test_support::test_app_state(
            crate::config::Config::default(),
            pool,
        ))
    }

    /// Envía `method` a `uri` contra `app` y devuelve `(status, json)`.
    ///
    /// Un cuerpo no-JSON (p. ej. el rechazo en texto plano del extractor `Query`)
    /// se materializa como [`serde_json::Value::Null`].
    async fn send_json(
        app: axum::Router,
        method: axum::http::Method,
        uri: &str,
    ) -> (StatusCode, serde_json::Value) {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("oneshot");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
        send_json(app, axum::http::Method::GET, uri).await
    }

    /// 1.10 — base vacía: envelope exacto `{items,total,page,page_size}`.
    #[tokio::test]
    async fn test_http_history_empty_envelope() {
        let (status, json) = get_json(http_app(pool()), "/api/history").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json,
            serde_json::json!({"items": [], "total": 0, "page": 1, "page_size": 20})
        );
    }

    /// 1.10 — `?page=2&page_size=50` sirve 50 items y el total completo.
    #[tokio::test]
    async fn test_http_history_second_page_of_fifty() {
        let pool = pool();
        let entries: Vec<UpdateHistoryEntry> = (0..120)
            .map(|i| {
                entry(
                    &format!("c{i:03}"),
                    &format!("2024-01-01T00:00:{:02}", i % 60),
                )
            })
            .collect();
        seed(&pool, &entries).await;

        let (status, json) = get_json(http_app(pool), "/api/history?page=2&page_size=50").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["page"], 2);
        assert_eq!(json["page_size"], 50);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(50));
        assert_eq!(json["total"], 120, "el total es el histórico, no la página");
    }

    /// 1.10 — `page_size=30` (fuera de {10,20,50}) → 400 con `{"error": ...}`.
    #[tokio::test]
    async fn test_http_history_page_size_30_is_bad_request() {
        let (status, json) = get_json(http_app(pool()), "/api/history?page_size=30").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "no debe ser 500");
        assert!(json["error"].is_string(), "cuerpo de error: {json}");
    }

    /// 1.10 — `page=0` → 400 desde el borde HTTP.
    #[tokio::test]
    async fn test_http_history_page_zero_is_bad_request() {
        let (status, _) = get_json(http_app(pool()), "/api/history?page=0").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    /// 1.10 — `page=abc` lo rechaza el extractor `Query` con 400 (no 500).
    #[tokio::test]
    async fn test_http_history_page_abc_is_bad_request() {
        let (status, _) = get_json(http_app(pool()), "/api/history?page=abc").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "no debe ser 500 ni panic");
    }

    /// 1.10 — `page_size=abc` → 400 desde el borde HTTP.
    #[tokio::test]
    async fn test_http_history_page_size_abc_is_bad_request() {
        let (status, _) = get_json(http_app(pool()), "/api/history?page_size=abc").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "no debe ser 500 ni panic");
    }

    /// 1.10 — escalar repetido a nivel HTTP: last-wins (`page=1&page=2` → 2).
    #[tokio::test]
    async fn test_http_history_repeated_scalar_last_wins() {
        let (status, json) = get_json(http_app(pool()), "/api/history?page=1&page=2").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["page"], 2);
    }

    /// 1.10 — filtro `q` a nivel HTTP con `total` posterior al filtrado.
    #[tokio::test]
    async fn test_http_history_q_filter() {
        let pool = pool();
        seed(
            &pool,
            &[
                entry("web", "2024-01-01T00:00:00"),
                entry("web-api", "2024-01-02T00:00:00"),
                entry("db", "2024-01-03T00:00:00"),
            ],
        )
        .await;

        let (status, json) =
            get_json(http_app(pool), "/api/history?page=1&page_size=10&q=web").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 2, "total tras filtrar");
        let items = json["items"].as_array().expect("items");
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|i| i["container"].as_str().unwrap_or_default().contains("web")),
            "solo deben viajar las coincidencias: {items:?}"
        );
    }

    /// 1.10 — `status` repetible con OR a nivel HTTP.
    #[tokio::test]
    async fn test_http_history_status_filter_or() {
        let pool = pool();
        let mut error = entry("a", "2024-01-01T00:00:00");
        error.status = "error".into();
        let mut rolled_back = entry("b", "2024-01-02T00:00:00");
        rolled_back.status = "rolled-back".into();
        let success = entry("c", "2024-01-03T00:00:00");
        seed(&pool, &[error, rolled_back, success]).await;

        let (status, json) = get_json(
            http_app(pool),
            "/api/history?status=error&status=rolled-back",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 2);
        let items = json["items"].as_array().expect("items");
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|i| i["status"] == "error" || i["status"] == "rolled-back"),
            "solo los estados pedidos: {items:?}"
        );
    }

    /// 1.10 — forma exacta del item: claves del contrato y sin `id`.
    #[tokio::test]
    async fn test_http_history_item_shape_excludes_id() {
        let pool = pool();
        seed(&pool, &[entry("web", "2024-01-01T00:00:00")]).await;

        let (status, json) = get_json(http_app(pool), "/api/history").await;
        assert_eq!(status, StatusCode::OK);
        let item = json["items"][0].as_object().expect("item objeto");
        let keys: std::collections::BTreeSet<&str> = item.keys().map(String::as_str).collect();
        let expected: std::collections::BTreeSet<&str> = [
            "container",
            "image",
            "old_digest",
            "new_digest",
            "timestamp",
            "status",
            "duration_ms",
            "attempts",
        ]
        .into_iter()
        .collect();
        assert_eq!(keys, expected, "claves del contrato; got: {keys:?}");
        assert!(!item.contains_key("id"), "el id no debe exponerse");
    }

    /// 1.10 — `DELETE /api/history` limpia y un `GET` posterior ve `total == 0`.
    #[tokio::test]
    async fn test_http_history_delete_clears() {
        let pool = pool();
        seed(&pool, &[entry("web", "2024-01-01T00:00:00")]).await;

        let (status, json) = send_json(
            http_app(pool.clone()),
            axum::http::Method::DELETE,
            "/api/history",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!({"status": "cleared"}));

        let (status, json) = get_json(http_app(pool), "/api/history").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 0);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(0));
    }
}
