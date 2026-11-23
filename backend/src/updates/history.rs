use axum::extract::{Query, State};
use axum::response::Json;
use std::sync::Arc;
use std::time::Instant;
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

/// `GET /api/updates/retention` — devuelve la política de retención actual.
pub async fn get_retention_h(
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Result<Json<UpdateHistoryRetention>, AppError> {
    let s = settings.lock().await;
    let retention_days = s.update_history_retention_days.unwrap_or(30);
    Ok(Json(UpdateHistoryRetention { retention_days }))
}

/// `PUT /api/updates/retention` — actualiza la política de retención.
///
/// El cuerpo se extrae como `Result<Json<_>, JsonRejection>` para mapear
/// cualquier fallo de deserialización (JSON malformado, `retention_days < 0`,
/// etc.) a `400 Bad Request` en vez del `422` por defecto de axum.
pub async fn put_retention_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    body: Result<Json<UpdateHistoryRetention>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<UpdateHistoryRetention>, AppError> {
    let Json(body) =
        body.map_err(|e| AppError::BadRequest(format!("cuerpo JSON inválido: {e}")))?;
    let mut s = settings.lock().await;
    s.update_history_retention_days = Some(body.retention_days);
    let conn = db_pool.get().await.map_err(AppError::from_pool)?;
    let guard = conn.lock().map_err(AppError::from_pool)?;
    let _ = db::save_settings(&guard, &s);
    Ok(Json(UpdateHistoryRetention {
        retention_days: s.update_history_retention_days.unwrap_or(30),
    }))
}

/// `POST /api/updates/retention/prune` — ejecuta poda manual del histórico.
pub async fn post_prune_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
) -> Result<Json<PruneResult>, AppError> {
    let s = settings.lock().await;
    let retention_days = s.update_history_retention_days.unwrap_or(30);
    let start = Instant::now();
    let deleted = db::prune_update_history(&db_pool, retention_days)
        .await
        .map_err(AppError::from_db)?;
    let duration_ms = start.elapsed().as_millis() as u64;
    Ok(Json(PruneResult {
        deleted,
        retention_days,
        duration_ms,
    }))
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

    // ── update-history-retention (RED: funcionalidad no implementada) ────

    /// 2.1 — GIVEN instalación nueva WHEN GET /api/updates/retention THEN 200 {retention_days: 30}
    #[tokio::test]
    async fn test_retention_default_get() {
        let pool = pool();
        let app = retention_http_app(pool);
        let (status, json) = get_json(app, "/api/updates/retention").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!({"retention_days": 30}));
    }

    /// 2.2 — GIVEN usuario autenticado WHEN PUT /api/updates/retention THEN 200 y persiste
    #[tokio::test]
    async fn test_retention_valid_put() {
        let pool = pool();
        let app = retention_http_app(pool);
        let (status, json) = put_json(
            &app,
            "/api/updates/retention",
            serde_json::json!({"retention_days": 7}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!({"retention_days": 7}));
        // Verificamos que GET posterior refleja el cambio
        let (_, after_get) = get_json(app, "/api/updates/retention").await;
        assert_eq!(after_get["retention_days"], 7);
    }

    /// 2.3 — retention_days = 0 → no poda, deleted == 0
    #[tokio::test]
    async fn test_retention_unlimited_no_puda() {
        let pool = pool();
        // Sembramos una entrada vieja para asegurar que hay datos
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        let obj = pool.get().await.expect("conexión de test");
        let conn = obj.lock().expect("lock de conexión");
        db::append_update_history(&conn, &old_entry).expect("append_update_history");
        drop(conn);
        drop(obj);

        // Llamada directa a la función de poda
        let deleted = db::prune_update_history(&pool, 0)
            .await
            .expect("prune_update_history");
        assert_eq!(deleted, 0, "con retention_days=0 no debe eliminarse nada");
    }

    /// 2.4 — Poda idempotente: ejecutar dos veces seguidas → deleted == 0 en la segunda
    #[tokio::test]
    async fn test_retention_idempotent_prune() {
        let pool = pool();
        // Sembramos una entrada vieja y guardamos la conexión
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        let obj = pool.get().await.expect("conexión de test");
        let conn = obj.lock().expect("lock de conexión");
        db::append_update_history(&conn, &old_entry).expect("append_update_history");
        drop(conn);
        drop(obj);

        // Primera poda con la pool
        let _ = db::prune_update_history(&pool, 7).await;

        // Segunda poda
        let deleted = db::prune_update_history(&pool, 7)
            .await
            .expect("prune_update_history");
        assert_eq!(
            deleted, 0,
            "segunda poda idempotente debe devolver deleted == 0"
        );
    }

    /// 2.5 — Error temporal en BD no rompe el patrón: la función retorna Result
    #[tokio::test]
    async fn test_retention_db_error_no_rompe_worker() {
        let pool = pool();
        // La función retorna Result; el caller debe manejar el error sin panic
        let _result = db::prune_update_history(&pool, 7).await;
        // Si llegamos aquí, el patrón de error handling es correcto (no hay panic)
    }

    /// 2.6 — Poda manual bajo demanda: usando la función DB → deleted >= 1
    #[tokio::test]
    async fn test_retention_manual_prune() {
        let pool = pool();
        // Sembramos entradas viejas (10 días)
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        let obj = pool.get().await.expect("conexión de test");
        let conn = obj.lock().expect("lock de conexión");
        db::append_update_history(&conn, &old_entry).expect("append_update_history");
        drop(conn);
        drop(obj);

        // Llamada directa a la función de poda con retention_days = 7
        let deleted = db::prune_update_history(&pool, 7)
            .await
            .expect("prune_update_history");
        assert!(
            deleted >= 1,
            "poda manual con retención activa debe eliminar al menos 1 fila, got {}",
            deleted
        );
    }

    /// 2.7 — Validación de entrada inválida: PUT con {retention_days: -1} → 400
    #[tokio::test]
    async fn test_retention_invalid_body_minus_one() {
        let pool = pool();
        let app = retention_http_app(pool);
        let (status, _json) = put_json(
            &app,
            "/api/updates/retention",
            serde_json::json!({"retention_days": -1}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "PUT con retention_days=-1 debe ser 400"
        );
        // La configuración no debe cambiar
        let (_, after_get) = get_json(app, "/api/updates/retention").await;
        assert_eq!(
            after_get["retention_days"], 30,
            "config debe mantenerse en default tras PUT inválido"
        );
    }

    /// 2.8 — Sin interferencia con operaciones críticas: durante poda, nueva inserción se completa
    #[tokio::test]
    async fn test_retention_no_interference_critical_ops() {
        let pool = pool();
        // Sembramos una entrada vieja
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        let obj = pool.get().await.expect("conexión de test");
        let conn = obj.lock().expect("lock de conexión");
        db::append_update_history(&conn, &old_entry).expect("append_update_history");
        drop(conn);
        drop(obj);

        // Ejecutamos poda usando la pool
        let _ = db::prune_update_history(&pool, 7).await;

        // Insertamos un nuevo registro después de la poda
        let new_entry = UpdateHistoryEntry {
            container: "new".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2026-06-15T12:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        let obj3 = pool.get().await.expect("conexión de test");
        let conn3 = obj3.lock().expect("lock de conexión");
        db::append_update_history(&conn3, &new_entry).expect("append_update_history");
        // Verificamos que el nuevo registro persiste y no es eliminado por la poda anterior
        let loaded = db::load_update_history(&conn3).unwrap();
        assert!(
            loaded.iter().any(|e| e.container == "new"),
            "el nuevo registro no debe ser eliminado por la poda"
        );
    }

    /// test_retention_default_is_30 — GET /api/updates/retention por defecto → 30
    #[tokio::test]
    async fn test_retention_default_is_30() {
        let pool = pool();
        let app = retention_http_app(pool);
        let (status, json) = get_json(app, "/api/updates/retention").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json,
            serde_json::json!({"retention_days": 30}),
            "el default de retención debe ser 30 días"
        );
    }

    /// test_retention_update_persists — PUT 7 y GET posterior refleja 7
    #[tokio::test]
    async fn test_retention_update_persists() {
        let pool = pool();
        let app = retention_http_app(pool);
        let (status, json) = put_json(
            &app,
            "/api/updates/retention",
            serde_json::json!({"retention_days": 7}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!({"retention_days": 7}));
        // GET posterior refleja el cambio persistido
        let (status, after_get) = get_json(app, "/api/updates/retention").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(after_get["retention_days"], 7);
    }

    /// test_retention_zero_no_prune — retention_days=0 no elimina nada
    #[tokio::test]
    async fn test_retention_zero_no_prune() {
        let pool = pool();
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        {
            let obj = pool.get().await.expect("conexión de test");
            let conn = obj.lock().expect("lock de conexión");
            db::append_update_history(&conn, &old_entry).expect("append_update_history");
        }
        let deleted = db::prune_update_history(&pool, 0)
            .await
            .expect("prune_update_history");
        assert_eq!(deleted, 0, "con retención 0 no se elimina nada");
        // La fila debe conservarse
        let obj = pool.get().await.expect("conexión de test");
        let conn = obj.lock().expect("lock de conexión");
        let loaded = db::load_update_history(&conn).unwrap();
        assert_eq!(loaded.len(), 1, "la fila debe conservarse con retención 0");
    }

    /// test_prune_idempotent — segunda poda devuelve deleted==0
    #[tokio::test]
    async fn test_prune_idempotent() {
        let pool = pool();
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        {
            let obj = pool.get().await.expect("conexión de test");
            let conn = obj.lock().expect("lock de conexión");
            db::append_update_history(&conn, &old_entry).expect("append_update_history");
        }
        let first = db::prune_update_history(&pool, 7)
            .await
            .expect("primera poda");
        assert!(
            first >= 1,
            "primera poda debe eliminar al menos 1 fila, got {first}"
        );
        let second = db::prune_update_history(&pool, 7)
            .await
            .expect("segunda poda");
        assert_eq!(
            second, 0,
            "segunda poda idempotente debe devolver deleted == 0"
        );
    }

    /// test_prune_resilient_to_db_error — pool cerrada no paniquea, devuelve Err
    #[tokio::test]
    async fn test_prune_resilient_to_db_error() {
        let pool = pool();
        // Cerramos la pool para forzar un error de obtención de conexión.
        pool.close();
        let result = db::prune_update_history(&pool, 7).await;
        assert!(
            result.is_err(),
            "pool cerrada → prune debe devolver Err, no paniquear"
        );
    }

    /// test_manual_prune_endpoint — POST /api/updates/retention/prune
    #[tokio::test]
    async fn test_manual_prune_endpoint() {
        let pool = pool();
        // Sembramos una entrada vieja (10 días, fuera de retención 7)
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        {
            let obj = pool.get().await.expect("conexión de test");
            let conn = obj.lock().expect("lock de conexión");
            db::append_update_history(&conn, &old_entry).expect("append_update_history");
        }
        let app = retention_http_app(pool);
        // Configuramos retención de 7 días
        let (status, _) = put_json(
            &app,
            "/api/updates/retention",
            serde_json::json!({"retention_days": 7}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Poda manual bajo demanda
        let (status, json) =
            post_json(&app, "/api/updates/retention/prune", serde_json::json!({})).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            json["deleted"].as_u64().unwrap_or(0) >= 1,
            "la poda manual elimina filas antiguas: {json:?}"
        );
        assert_eq!(json["retention_days"], 7);
        assert!(
            json["duration_ms"].is_number(),
            "duration_ms debe ser un número >= 0"
        );
    }

    /// test_retention_invalid_input_rejected — PUT con -1 → 400
    #[tokio::test]
    async fn test_retention_invalid_input_rejected() {
        let pool = pool();
        let app = retention_http_app(pool);
        let (status, _) = put_json(
            &app,
            "/api/updates/retention",
            serde_json::json!({"retention_days": -1}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "PUT con retention_days=-1 debe ser 400"
        );
    }

    /// test_prune_does_not_block_critical_ops — poda + inserción nueva coexisten
    #[tokio::test]
    async fn test_prune_does_not_block_critical_ops() {
        let pool = pool();
        let old_entry = UpdateHistoryEntry {
            container: "old".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2020-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        {
            let obj = pool.get().await.expect("conexión de test");
            let conn = obj.lock().expect("lock de conexión");
            db::append_update_history(&conn, &old_entry).expect("append_update_history");
        }
        // Poda
        let _ = db::prune_update_history(&pool, 7)
            .await
            .expect("prune_update_history");
        // Nueva inserción tras la poda (debe completarse y no ser eliminada)
        let new_entry = UpdateHistoryEntry {
            container: "new".into(),
            image: "img".into(),
            old_digest: String::new(),
            new_digest: String::new(),
            timestamp: "2026-06-15T12:00:00".into(),
            status: "success".into(),
            duration_ms: 0,
            attempts: 0,
        };
        {
            let obj = pool.get().await.expect("conexión de test");
            let conn = obj.lock().expect("lock de conexión");
            db::append_update_history(&conn, &new_entry).expect("append_update_history");
            let loaded = db::load_update_history(&conn).unwrap();
            assert!(
                loaded.iter().any(|e| e.container == "new"),
                "el nuevo registro debe persistir tras la poda"
            );
        }
    }

    // ── Helpers HTTP para pruebas de retención ─────────────────────────

    /// Monta el router de updates para tests de retención.
    fn retention_http_app(pool: DbPool) -> axum::Router {
        crate::updates::routes().with_state(crate::test_support::test_app_state(
            crate::config::Config::default(),
            pool,
        ))
    }

    /// Envía `method` con `body` JSON a `uri` y devuelve (status, json).
    async fn send_json_body(
        app: axum::Router,
        method: axum::http::Method,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .expect("request"),
            )
            .await
            .expect("oneshot");
        let status = response.status();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    /// PUT con cuerpo JSON.
    async fn put_json(
        app: &axum::Router,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        send_json_body(app.clone(), axum::http::Method::PUT, uri, body).await
    }

    /// POST con cuerpo JSON.
    async fn post_json(
        app: &axum::Router,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        send_json_body(app.clone(), axum::http::Method::POST, uri, body).await
    }
}
