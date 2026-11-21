use axum::{
    extract::State,
    response::Json,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::state::AppState;

// ── Update Check Config ──────────────────────────────────────

/// Defaults canónicos de la configuración de updates
/// (`GET /api/update-check/defaults`). No depende de `Settings`: expone las
/// consts tal cual, sin aplicar la lógica efectiva del GET existente.
async fn get_update_defaults_h() -> Json<UpdateDefaults> {
    Json(UpdateDefaults::canonical())
}

async fn get_update_check_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<UpdateCheckConfig> {
    let s = settings.lock().await;
    let cron = s.effective_update_check_cron();
    let next_run_at = compute_next_run(&cron);
    Json(UpdateCheckConfig {
        cron,
        enabled: s.update_check_enabled.unwrap_or(false),
        notify: s.update_check_notify.unwrap_or(false),
        last_run_at: s.update_check_last_run_at.clone(),
        next_run_at,
        pull_timeout_secs: Some(s.effective_pull_timeout_secs()),
        check_interval_ms: Some(s.effective_check_interval_ms()),
        update_max_start_attempts: s.update_max_start_attempts,
        update_start_attempt_interval_secs: s.update_start_attempt_interval_secs,
        update_health_window_secs: s.update_health_window_secs,
    })
}

fn compute_next_run(cron_expr: &str) -> Option<String> {
    if cron_expr.is_empty() {
        return None;
    }
    let schedule = format!("0 {}", cron_expr).parse::<cron::Schedule>().ok()?;
    let now = crate::timezone::now();
    Some(
        schedule
            .upcoming(now.timezone())
            .next()?
            .format("%Y-%m-%dT%H:%M:%S%:z")
            .to_string(),
    )
}

async fn put_update_check_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<UpdateCheckConfig>,
) -> Result<Json<UpdateCheckConfig>, AppError> {
    // validate-cron-on-put: rechazar un cron inválido ANTES de mutar el estado
    // en memoria o persistirlo.
    if !crate::timezone::is_valid_cron(&body.cron) {
        return Err(AppError::BadRequest(
            "expresión cron inválida (se esperan 5 campos, p. ej. \"0 6 * * *\")".into(),
        ));
    }
    // Conexión antes que estado: no retener `settings.lock()` mientras se espera
    // una conexión de la pool.
    let conn = db_pool.get().await.unwrap();
    let mut s = settings.lock().await;
    s.update_check_cron = Some(body.cron.clone());
    s.update_check_enabled = Some(body.enabled);
    s.update_check_notify = Some(body.notify);
    if body.pull_timeout_secs.is_some() {
        s.pull_timeout_secs = body.pull_timeout_secs;
    }
    if body.check_interval_ms.is_some() {
        s.check_interval_ms = body.check_interval_ms;
    }
    // Guarda de actualización: persistir los 3 parámetros globales.
    s.update_max_start_attempts = body.update_max_start_attempts;
    s.update_start_attempt_interval_secs = body.update_start_attempt_interval_secs;
    s.update_health_window_secs = body.update_health_window_secs;
    let _ = db::save_settings(&conn.lock().unwrap(), &s);
    let next_run_at = compute_next_run(&body.cron);
    Ok(Json(UpdateCheckConfig {
        cron: body.cron,
        enabled: body.enabled,
        notify: body.notify,
        last_run_at: s.update_check_last_run_at.clone(),
        next_run_at,
        pull_timeout_secs: s.pull_timeout_secs,
        check_interval_ms: s.check_interval_ms,
        update_max_start_attempts: s.update_max_start_attempts,
        update_start_attempt_interval_secs: s.update_start_attempt_interval_secs,
        update_health_window_secs: s.update_health_window_secs,
    }))
}

// ── Update Policies ─────────────────────────────────────────

async fn get_update_policies_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
) -> Json<Vec<UpdatePolicy>> {
    let list = policies.lock().await;
    Json(list.clone())
}

async fn put_update_policy_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    State(db_pool): State<DbPool>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<UpdatePolicyReq>,
) -> Json<UpdatePolicy> {
    let policy = UpdatePolicy {
        container: name.clone(),
        action: body.action,
        cleanup_old_image: body.cleanup_old_image,
        rollback_on_failure: body.rollback_on_failure,
        notify_events: body.notify_events,
        // Overrides de la guarda de arranque sano (opcionales).
        max_start_attempts: body.max_start_attempts,
        health_window_secs: body.health_window_secs,
    };
    {
        let mut list = policies.lock().await;
        if let Some(existing) = list.iter_mut().find(|p| p.container == name) {
            existing.action = policy.action.clone();
            existing.cleanup_old_image = policy.cleanup_old_image;
            existing.rollback_on_failure = policy.rollback_on_failure;
            existing.notify_events = policy.notify_events;
            existing.max_start_attempts = policy.max_start_attempts;
            existing.health_window_secs = policy.health_window_secs;
        } else {
            list.push(policy.clone());
        }
    }
    let conn = db_pool.get().await.unwrap();
    let _ = db::save_update_policy(&conn.lock().unwrap(), &policy);
    Json(policy)
}

async fn delete_update_policy_h(
    State(policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    State(db_pool): State<DbPool>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    {
        let mut list = policies.lock().await;
        list.retain(|p| p.container != name);
    }
    let conn = db_pool.get().await.unwrap();
    let _ = db::delete_update_policy(&conn.lock().unwrap(), &name);
    Json(serde_json::json!({"status": "deleted", "container": name}))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DefaultUpdatePolicy {
    action: String,
    cleanup_old_image: bool,
    rollback_on_failure: bool,
    /// Override de la política por defecto del nº máximo de intentos de arranque.
    #[serde(default)]
    max_start_attempts: Option<u32>,
    /// Override de la política por defecto de la ventana estable de salud (segundos).
    #[serde(default)]
    health_window_secs: Option<u64>,
}

async fn get_default_update_policy_h(
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<DefaultUpdatePolicy> {
    let s = settings.lock().await;
    Json(DefaultUpdatePolicy {
        action: s
            .default_update_action
            .clone()
            .unwrap_or_else(|| "pull-restart".into()),
        cleanup_old_image: s.default_cleanup_old_image.unwrap_or(false),
        rollback_on_failure: s.default_rollback_on_failure.unwrap_or(false),
        max_start_attempts: s.default_max_start_attempts,
        health_window_secs: s.default_health_window_secs,
    })
}

async fn put_default_update_policy_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<DefaultUpdatePolicy>,
) -> Json<DefaultUpdatePolicy> {
    // Conexión antes que estado: no retener `settings.lock()` mientras se espera
    // una conexión de la pool.
    let conn = db_pool.get().await.unwrap();
    let mut s = settings.lock().await;
    s.default_update_action = Some(body.action.clone());
    s.default_cleanup_old_image = Some(body.cleanup_old_image);
    s.default_rollback_on_failure = Some(body.rollback_on_failure);
    s.default_max_start_attempts = body.max_start_attempts;
    s.default_health_window_secs = body.health_window_secs;
    let _ = db::save_settings(&conn.lock().unwrap(), &s);
    Json(body)
}

// ── Export / Import ──────────────────────────────────────────

async fn export_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
) -> Json<serde_json::Value> {
    // Conexión antes que estado: adquirir la conexión antes de los locks de
    // `settings` y `update_history` para no retenerlos mientras se espera.
    let conn = db_pool.get().await.unwrap();
    let sett = settings.lock().await;
    let hist = update_history.lock().await;
    let _ = db::save_settings(&conn.lock().unwrap(), &sett);
    // Redact secrets: never emit notification tokens in the exported payload.
    let mut redacted = sett.clone();
    redacted.telegram_token = None;
    redacted.matrix_token = None;
    redacted.valet_token = None;
    redacted.webhook_url = None;
    Json(serde_json::json!({
        "version": 1,
        "exported_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "settings": redacted,
        "update_history": *hist,
    }))
}

#[derive(serde::Deserialize)]
struct ImportPayload {
    settings: Settings,
}

async fn import_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<ImportPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    // validate-cron-on-put: un cron presente y no vacío debe ser válido ANTES de
    // mutar el estado en memoria o persistirlo. `None` o `""` = sin override;
    // cualquier otro valor (p. ej. solo espacios) se valida igual que el PUT.
    if let Some(cron) = body.settings.update_check_cron.as_deref() {
        if !cron.is_empty() && !crate::timezone::is_valid_cron(cron) {
            return Err(AppError::BadRequest(
                "expresión cron inválida en la configuración importada (se esperan 5 campos, p. ej. \"0 6 * * *\")"
                    .into(),
            ));
        }
    }
    // Conexión antes que estado: adquirir la conexión antes de mutar el estado,
    // manteniendo mutación + persistencia en una única sección crítica.
    let conn = db_pool.get().await.unwrap();
    {
        let mut st = settings.lock().await;
        *st = body.settings;
        let _ = db::save_settings(&conn.lock().unwrap(), &st);
    }
    Ok(Json(serde_json::json!({"status": "imported"})))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/update-check/config",
            get(get_update_check_config_h).put(put_update_check_config_h),
        )
        .route("/api/update-check/defaults", get(get_update_defaults_h))
        .route("/api/update-policies", get(get_update_policies_h))
        .route(
            "/api/update-policies/default",
            get(get_default_update_policy_h).put(put_default_update_policy_h),
        )
        .route(
            "/api/update-policies/{name}",
            put(put_update_policy_h).delete(delete_update_policy_h),
        )
        .route("/api/admin/export", get(export_config_h))
        .route("/api/admin/import", post(import_config_h))
}

use axum::routing::post;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

    // ── Update Check Config ─────────────────────────────────

    #[tokio::test]
    async fn test_get_update_check_config_defaults() {
        let settings = Arc::new(Mutex::new(Settings::default()));
        let result: Json<UpdateCheckConfig> = get_update_check_config_h(State(settings)).await;
        assert_eq!(result.0.cron, DEFAULT_UPDATE_CHECK_CRON);
        assert!(!result.0.enabled);
        assert!(!result.0.notify);
        // El GET sirve los valores efectivos (consts), no `None`.
        assert_eq!(result.0.pull_timeout_secs, Some(DEFAULT_PULL_TIMEOUT_SECS));
        assert_eq!(result.0.check_interval_ms, Some(DEFAULT_CHECK_INTERVAL_MS));
    }

    #[tokio::test]
    async fn test_get_update_defaults_returns_canonical() {
        // `GET /api/update-check/defaults` ⇒ 200 (Json) con las consts canónicas.
        let Json(defaults) = get_update_defaults_h().await;
        assert_eq!(defaults.cron, "0 6 * * *");
        assert_eq!(defaults.pull_timeout_secs, 600);
        assert_eq!(defaults.check_interval_ms, 2000);
        // Defaults de la guarda de arranque (consts canónicas).
        assert_eq!(defaults.update_max_start_attempts, 3);
        assert_eq!(defaults.update_start_attempt_interval_secs, 10);
        assert_eq!(defaults.update_health_window_secs, 30);
    }

    #[tokio::test]
    async fn test_put_update_check_config() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let config = UpdateCheckConfig {
            cron: "0 0 * * *".into(),
            enabled: true,
            notify: true,
            last_run_at: None,
            next_run_at: None,
            pull_timeout_secs: None,
            check_interval_ms: None,
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
        };
        let result: Json<UpdateCheckConfig> =
            put_update_check_config_h(State(settings.clone()), State(db_pool), Json(config))
                .await
                .expect("un cron válido debe aceptarse");
        assert_eq!(result.0.cron, "0 0 * * *");
        assert!(result.0.enabled);
        assert!(result.0.notify);
        let s = settings.lock().await;
        assert_eq!(s.update_check_cron.as_deref(), Some("0 0 * * *"));
        assert_eq!(s.update_check_enabled, Some(true));
    }

    /// Construye un `UpdateCheckConfig` de prueba con el cron indicado.
    fn test_update_check_config(cron: &str) -> UpdateCheckConfig {
        UpdateCheckConfig {
            cron: cron.into(),
            enabled: true,
            notify: false,
            last_run_at: None,
            next_run_at: None,
            pull_timeout_secs: None,
            check_interval_ms: None,
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
        }
    }

    /// Siembra la tabla `settings` con un cron previo persistido, para poder
    /// comprobar que un PUT rechazado no la sobrescribe.
    async fn seed_persisted_cron(db_pool: &DbPool, cron: &str) {
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        let mut s = db::load_settings(&guard).unwrap();
        s.update_check_cron = Some(cron.into());
        db::save_settings(&guard, &s).unwrap();
    }

    /// Lee el cron guardado en la tabla `settings` (fuente de verdad en disco).
    async fn persisted_cron(db_pool: &DbPool) -> Option<String> {
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        db::load_settings(&guard).unwrap().update_check_cron
    }

    /// validate-cron-on-put: un cron inválido ⇒ 400 y ni la memoria ni los
    /// settings persistidos cambian.
    #[tokio::test]
    async fn test_put_update_check_config_invalid_cron_rejected() {
        let db_pool = db::test_pool();
        // Valor previo persistido: el PUT fallido no debe tocarlo.
        seed_persisted_cron(&db_pool, "0 6 * * *").await;
        let settings = Arc::new(Mutex::new(Settings::default()));
        {
            let mut s = settings.lock().await;
            s.update_check_cron = Some("0 6 * * *".into());
            s.update_check_enabled = Some(true);
        }
        let before = {
            let s = settings.lock().await;
            (s.update_check_cron.clone(), s.update_check_enabled)
        };
        let before_db = persisted_cron(&db_pool).await;

        let result = put_update_check_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(test_update_check_config("not a cron")),
        )
        .await;
        match result {
            Err(AppError::BadRequest(_)) => {}
            other => panic!("se esperaba 400 (BadRequest), obtenido: {:?}", other),
        }

        let after = {
            let s = settings.lock().await;
            (s.update_check_cron.clone(), s.update_check_enabled)
        };
        assert_eq!(
            before, after,
            "el estado en memoria no debe mutarse al rechazar"
        );
        let after_db = persisted_cron(&db_pool).await;
        assert_eq!(
            before_db, after_db,
            "no debe escribirse en la base tras un PUT rechazado"
        );
        assert_eq!(
            after_db.as_deref(),
            Some("0 6 * * *"),
            "el cron previo persistido debe quedar intacto"
        );
    }

    /// validate-cron-on-put: cron vacío ⇒ 400, sin tocar memoria ni disco.
    #[tokio::test]
    async fn test_put_update_check_config_empty_cron_rejected() {
        let db_pool = db::test_pool();
        seed_persisted_cron(&db_pool, "0 6 * * *").await;
        let settings = Arc::new(Mutex::new(Settings::default()));
        let before_db = persisted_cron(&db_pool).await;

        let result = put_update_check_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(test_update_check_config("")),
        )
        .await;
        match result {
            Err(AppError::BadRequest(_)) => {}
            other => panic!("se esperaba 400 (BadRequest), obtenido: {:?}", other),
        }
        {
            let s = settings.lock().await;
            assert_eq!(
                s.update_check_cron, None,
                "no debe mutar la memoria con cron vacío"
            );
        }
        let after_db = persisted_cron(&db_pool).await;
        assert_eq!(before_db, after_db, "no debe escribirse en la base");
        assert_eq!(
            after_db.as_deref(),
            Some("0 6 * * *"),
            "el cron previo persistido debe quedar intacto"
        );
    }

    /// validate-cron-on-put: cron con 6 campos ⇒ 400 (solo formato canónico de
    /// 5), sin tocar memoria ni disco.
    #[tokio::test]
    async fn test_put_update_check_config_six_field_cron_rejected() {
        let db_pool = db::test_pool();
        seed_persisted_cron(&db_pool, "*/15 * * * *").await;
        let settings = Arc::new(Mutex::new(Settings::default()));
        let before_db = persisted_cron(&db_pool).await;

        let result = put_update_check_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(test_update_check_config("0 0 6 * * *")),
        )
        .await;
        match result {
            Err(AppError::BadRequest(_)) => {}
            other => panic!("se esperaba 400 (BadRequest), obtenido: {:?}", other),
        }
        {
            let s = settings.lock().await;
            assert_eq!(
                s.update_check_cron, None,
                "no debe mutar la memoria con 6 campos"
            );
        }
        let after_db = persisted_cron(&db_pool).await;
        assert_eq!(before_db, after_db, "no debe escribirse en la base");
        assert_eq!(
            after_db.as_deref(),
            Some("*/15 * * * *"),
            "el cron previo persistido debe quedar intacto"
        );
    }

    /// validate-cron-on-put: cron válido de 5 campos ⇒ 200 y persiste.
    #[tokio::test]
    async fn test_put_update_check_config_valid_five_field_cron_accepted() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let result = put_update_check_config_h(
            State(settings.clone()),
            State(db_pool),
            Json(test_update_check_config("0 6 * * *")),
        )
        .await
        .expect("un cron válido debe aceptarse");
        assert_eq!(result.0.cron, "0 6 * * *");
        assert!(
            result.0.next_run_at.is_some(),
            "next_run_at debe calcularse"
        );
        let s = settings.lock().await;
        assert_eq!(s.update_check_cron.as_deref(), Some("0 6 * * *"));
    }

    // ── Update Policies ─────────────────────────────────────

    /// update-rollback-guard: PUT/GET del config global persiste y sirve los 3
    /// campos de la guarda (memoria + settings en DB).
    #[tokio::test]
    async fn test_update_check_config_guard_roundtrip() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let config = UpdateCheckConfig {
            cron: "0 */6 * * *".into(),
            enabled: true,
            notify: false,
            last_run_at: None,
            next_run_at: None,
            pull_timeout_secs: None,
            check_interval_ms: None,
            update_max_start_attempts: 4,
            update_start_attempt_interval_secs: 15,
            update_health_window_secs: 45,
        };
        let _: Json<UpdateCheckConfig> = put_update_check_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(config),
        )
        .await
        .expect("cron válido debe aceptarse");

        // Persistido en el estado en memoria.
        {
            let s = settings.lock().await;
            assert_eq!(s.update_max_start_attempts, 4);
            assert_eq!(s.update_start_attempt_interval_secs, 15);
            assert_eq!(s.update_health_window_secs, 45);
        }

        // Servido por el GET.
        let got: Json<UpdateCheckConfig> = get_update_check_config_h(State(settings)).await;
        assert_eq!(got.0.update_max_start_attempts, 4);
        assert_eq!(got.0.update_start_attempt_interval_secs, 15);
        assert_eq!(got.0.update_health_window_secs, 45);

        // Persistido en la tabla `settings`.
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        let loaded = db::load_settings(&guard).unwrap();
        assert_eq!(loaded.update_max_start_attempts, 4);
        assert_eq!(loaded.update_start_attempt_interval_secs, 15);
        assert_eq!(loaded.update_health_window_secs, 45);
    }

    /// update-rollback-guard: la política acepta y guarda los overrides de la guarda.
    #[tokio::test]
    async fn test_put_update_policy_persists_guard_overrides() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(Vec::new()));
        let req = UpdatePolicyReq {
            action: UpdateAction::PullRestart,
            cleanup_old_image: false,
            rollback_on_failure: true,
            notify_events: true,
            max_start_attempts: Some(5),
            health_window_secs: Some(60),
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
            Json(req),
        )
        .await;
        assert_eq!(result.0.max_start_attempts, Some(5));
        assert_eq!(result.0.health_window_secs, Some(60));
        let list = policies.lock().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].max_start_attempts, Some(5));
        assert_eq!(list[0].health_window_secs, Some(60));
    }

    /// apply-default-guard-overrides: PUT/GET de la política por defecto
    /// persisten los overrides de la guarda (memoria + DB) y los devuelven.
    #[tokio::test]
    async fn test_default_update_policy_guard_roundtrip() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let body = DefaultUpdatePolicy {
            action: "pull-restart".into(),
            cleanup_old_image: false,
            rollback_on_failure: false,
            max_start_attempts: Some(5),
            health_window_secs: Some(60),
        };
        let put: Json<DefaultUpdatePolicy> = put_default_update_policy_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(body),
        )
        .await;
        assert_eq!(put.0.max_start_attempts, Some(5));
        assert_eq!(put.0.health_window_secs, Some(60));

        // Persistido en el estado en memoria.
        {
            let s = settings.lock().await;
            assert_eq!(s.default_max_start_attempts, Some(5));
            assert_eq!(s.default_health_window_secs, Some(60));
        }

        // Servido por el GET.
        let got: Json<DefaultUpdatePolicy> = get_default_update_policy_h(State(settings)).await;
        assert_eq!(got.0.max_start_attempts, Some(5));
        assert_eq!(got.0.health_window_secs, Some(60));

        // Persistido en la tabla `settings`.
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        let loaded = db::load_settings(&guard).unwrap();
        assert_eq!(loaded.default_max_start_attempts, Some(5));
        assert_eq!(loaded.default_health_window_secs, Some(60));
    }

    /// apply-default-guard-overrides: un `null` en el PUT limpia el override.
    #[tokio::test]
    async fn test_default_update_policy_guard_overrides_can_be_cleared() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings {
            default_max_start_attempts: Some(5),
            default_health_window_secs: Some(60),
            ..Default::default()
        }));
        let body = DefaultUpdatePolicy {
            action: "pull-restart".into(),
            cleanup_old_image: false,
            rollback_on_failure: false,
            max_start_attempts: None,
            health_window_secs: None,
        };
        let _: Json<DefaultUpdatePolicy> =
            put_default_update_policy_h(State(settings.clone()), State(db_pool), Json(body)).await;
        let s = settings.lock().await;
        assert_eq!(s.default_max_start_attempts, None);
        assert_eq!(s.default_health_window_secs, None);
    }

    /// settings-lock-scope: `PUT /api/update-policies/default` NO debe retener el
    /// mutex de `Settings` mientras espera una conexión de la pool saturada.
    #[tokio::test]
    async fn test_put_default_update_policy_does_not_hold_settings_lock_while_waiting_for_db() {
        let db_pool = db::test_pool_with_max_size(1);
        // Satura la única conexión: el handler quedará esperando en `db_pool.get()`.
        let _held = db_pool.get().await.expect("get conn");

        let settings = Arc::new(Mutex::new(Settings::default()));
        let body = DefaultUpdatePolicy {
            action: "pull-restart".into(),
            cleanup_old_image: false,
            rollback_on_failure: false,
            max_start_attempts: None,
            health_window_secs: None,
        };

        let handler_settings = settings.clone();
        let handler_pool = db_pool.clone();
        crate::test_support::assert_settings_lock_free_while_db_pool_saturated(
            settings.clone(),
            async move {
                let _ = put_default_update_policy_h(
                    State(handler_settings),
                    State(handler_pool),
                    Json(body),
                )
                .await;
            },
        )
        .await;
    }

    /// settings-lock-scope: `PUT /api/update-check/config` NO debe retener el
    /// mutex de `Settings` mientras espera una conexión de la pool saturada.
    #[tokio::test]
    async fn test_put_update_check_config_does_not_hold_settings_lock_while_waiting_for_db() {
        let db_pool = db::test_pool_with_max_size(1);
        // Satura la única conexión: el handler quedará esperando en `db_pool.get()`.
        let _held = db_pool.get().await.expect("get conn");

        let settings = Arc::new(Mutex::new(Settings::default()));
        let body = UpdateCheckConfig {
            cron: "0 6 * * *".into(),
            enabled: true,
            notify: false,
            last_run_at: None,
            next_run_at: None,
            pull_timeout_secs: None,
            check_interval_ms: None,
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
        };

        let handler_settings = settings.clone();
        let handler_pool = db_pool.clone();
        crate::test_support::assert_settings_lock_free_while_db_pool_saturated(
            settings.clone(),
            async move {
                let _ = put_update_check_config_h(
                    State(handler_settings),
                    State(handler_pool),
                    Json(body),
                )
                .await;
            },
        )
        .await;
    }

    /// settings-lock-scope: `GET /api/admin/export` NO debe retener ni el mutex de
    /// `Settings` ni el de `update_history` mientras espera una conexión de la pool
    /// saturada.
    #[tokio::test]
    async fn test_export_config_does_not_hold_settings_lock_while_waiting_for_db() {
        let db_pool = db::test_pool_with_max_size(1);
        // Satura la única conexión: el handler quedará esperando en `db_pool.get()`.
        let _held = db_pool.get().await.expect("get conn");

        let settings = Arc::new(Mutex::new(Settings::default()));
        let update_history = Arc::new(Mutex::new(Vec::<UpdateHistoryEntry>::new()));

        let handler_settings = settings.clone();
        let handler_history = update_history.clone();
        let handler_pool = db_pool.clone();
        crate::test_support::assert_settings_and_extra_lock_free_while_db_pool_saturated(
            settings.clone(),
            update_history.clone(),
            "update_history.lock()",
            async move {
                let _ = export_config_h(
                    State(handler_settings),
                    State(handler_history),
                    State(handler_pool),
                )
                .await;
            },
        )
        .await;
    }

    /// settings-lock-scope: `POST /api/admin/import` NO debe retener el mutex de
    /// `Settings` mientras espera una conexión de la pool saturada.
    #[tokio::test]
    async fn test_import_config_does_not_hold_settings_lock_while_waiting_for_db() {
        let db_pool = db::test_pool_with_max_size(1);
        // Satura la única conexión: el handler quedará esperando en `db_pool.get()`.
        let _held = db_pool.get().await.expect("get conn");

        let settings = Arc::new(Mutex::new(Settings::default()));
        let payload = ImportPayload {
            settings: Settings::default(),
        };

        let handler_settings = settings.clone();
        let handler_pool = db_pool.clone();
        crate::test_support::assert_settings_lock_free_while_db_pool_saturated(
            settings.clone(),
            async move {
                // El handler devuelve `Result`; el helper espera `()`, así que se
                // descarta el resultado dentro del future.
                let _ =
                    import_config_h(State(handler_settings), State(handler_pool), Json(payload))
                        .await;
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_get_update_policies_empty() {
        let policies = Arc::new(Mutex::new(Vec::new()));
        let result: Json<Vec<UpdatePolicy>> = get_update_policies_h(State(policies)).await;
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_put_update_policy() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(Vec::new()));
        let req = UpdatePolicyReq {
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: true,
            notify_events: false,
            max_start_attempts: None,
            health_window_secs: None,
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
            Json(req),
        )
        .await;
        assert_eq!(result.0.container, "nginx");
        assert_eq!(result.0.action, UpdateAction::PullRestart);
        assert!(result.0.cleanup_old_image);
        assert!(result.0.rollback_on_failure);
        let list = policies.lock().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].action, UpdateAction::PullRestart);
    }

    #[tokio::test]
    async fn test_put_update_policy_overwrites() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(vec![UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::None,
            cleanup_old_image: false,
            rollback_on_failure: false,
            notify_events: false,
            max_start_attempts: None,
            health_window_secs: None,
        }]));
        let req = UpdatePolicyReq {
            action: UpdateAction::Pull,
            cleanup_old_image: false,
            rollback_on_failure: false,
            notify_events: false,
            max_start_attempts: None,
            health_window_secs: None,
        };
        let result: Json<UpdatePolicy> = put_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
            Json(req),
        )
        .await;
        assert_eq!(result.0.action, UpdateAction::Pull);
        let list = policies.lock().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].action, UpdateAction::Pull);
    }

    #[tokio::test]
    async fn test_delete_update_policy() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(vec![UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: false,
            notify_events: false,
            max_start_attempts: None,
            health_window_secs: None,
        }]));
        let result: Json<serde_json::Value> = delete_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nginx".into()),
        )
        .await;
        assert_eq!(result.0["status"], "deleted");
        let list = policies.lock().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_delete_update_policy_nonexistent() {
        let db_pool = db::test_pool();
        let policies = Arc::new(Mutex::new(Vec::new()));
        let result: Json<serde_json::Value> = delete_update_policy_h(
            State(policies.clone()),
            State(db_pool),
            axum::extract::Path("nonexistent".into()),
        )
        .await;
        assert_eq!(result.0["status"], "deleted");
    }

    #[tokio::test]
    async fn test_get_update_policies_multiple() {
        let policies = Arc::new(Mutex::new(vec![
            UpdatePolicy {
                container: "nginx".into(),
                action: UpdateAction::Pull,
                cleanup_old_image: false,
                rollback_on_failure: false,
                notify_events: false,
                max_start_attempts: None,
                health_window_secs: None,
            },
            UpdatePolicy {
                container: "redis".into(),
                action: UpdateAction::None,
                cleanup_old_image: false,
                rollback_on_failure: false,
                notify_events: false,
                max_start_attempts: None,
                health_window_secs: None,
            },
        ]));
        let result: Json<Vec<UpdatePolicy>> = get_update_policies_h(State(policies)).await;
        assert_eq!(result.0.len(), 2);
    }

    // ── Export / Import ──────────────────────────────────────

    #[tokio::test]
    async fn test_export_config_empty() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));

        let result: Json<serde_json::Value> =
            export_config_h(State(settings), State(update_history), State(db_pool)).await;

        assert_eq!(result.0["version"], 1);
        assert!(result.0["exported_at"].is_string());
        assert!(result.0["settings"].is_object());
    }

    #[tokio::test]
    async fn test_export_config_redacts_tokens() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings {
            telegram_token: Some("tg".into()),
            matrix_token: Some("mx".into()),
            valet_token: Some("val".into()),
            webhook_url: Some("https://hooks.test/secret".into()),
            telegram_chat_id: Some("chat".into()),
            valet_url: Some("https://v".into()),
            matrix_homeserver: Some("https://matrix.test".into()),
            matrix_room: Some("!room:matrix.test".into()),
            update_check_cron: Some("0 */6 * * *".into()),
            ..Default::default()
        }));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
            Arc::new(Mutex::new(vec![UpdateHistoryEntry {
                container: "nginx".into(),
                image: "nginx:latest".into(),
                old_digest: "a".into(),
                new_digest: "b".into(),
                timestamp: "2026-01-01T00:00:00".into(),
                status: "success".into(),
                duration_ms: 10,
                attempts: 0,
            }]));

        let result: Json<serde_json::Value> =
            export_config_h(State(settings), State(update_history), State(db_pool)).await;

        assert!(result.0["settings"]["telegram_token"].is_null());
        assert!(result.0["settings"]["matrix_token"].is_null());
        assert!(result.0["settings"]["valet_token"].is_null());
        assert!(result.0["settings"]["webhook_url"].is_null());
        assert_eq!(result.0["settings"]["telegram_chat_id"], "chat");
        assert_eq!(result.0["settings"]["valet_url"], "https://v");
        assert_eq!(
            result.0["settings"]["matrix_homeserver"],
            "https://matrix.test"
        );
        assert_eq!(result.0["settings"]["matrix_room"], "!room:matrix.test");
        assert_eq!(result.0["settings"]["update_check_cron"], "0 */6 * * *");
        assert_eq!(result.0["update_history"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_import_config_replaces_state() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings {
                telegram_token: None,
                telegram_chat_id: Some("123".into()),
                ..Default::default()
            },
        };

        let result = import_config_h(State(settings.clone()), State(db_pool), Json(payload))
            .await
            .expect("un settings sin cron debe aceptarse");
        assert_eq!(result.0["status"], "imported");

        let st = settings.lock().await;
        assert!(st.telegram_token.is_none());
        assert_eq!(st.telegram_chat_id.as_deref(), Some("123"));
    }

    #[tokio::test]
    async fn test_import_config_overwrites_existing() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings::default(),
        };

        let _ = import_config_h(State(settings.clone()), State(db_pool), Json(payload))
            .await
            .expect("un settings por defecto debe aceptarse");
    }

    /// validate-cron-on-put (ampliación): importar un settings con un cron
    /// inválido ⇒ 400 y ni la memoria ni el estado persistido cambian.
    #[tokio::test]
    async fn test_import_config_invalid_cron_rejected() {
        let db_pool = db::test_pool();
        seed_persisted_cron(&db_pool, "0 6 * * *").await;
        let settings = Arc::new(Mutex::new(Settings {
            update_check_cron: Some("0 6 * * *".into()),
            telegram_chat_id: Some("keep".into()),
            ..Default::default()
        }));
        let before_db = persisted_cron(&db_pool).await;

        let payload = ImportPayload {
            settings: Settings {
                update_check_cron: Some("not a cron".into()),
                telegram_chat_id: Some("imported".into()),
                ..Default::default()
            },
        };

        let result = import_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(payload),
        )
        .await;
        match result {
            Err(AppError::BadRequest(_)) => {}
            other => panic!("se esperaba 400 (BadRequest), obtenido: {other:?}"),
        }

        // El estado en memoria no debe mutarse al rechazar.
        {
            let s = settings.lock().await;
            assert_eq!(
                s.update_check_cron.as_deref(),
                Some("0 6 * * *"),
                "el cron en memoria debe quedar intacto"
            );
            assert_eq!(
                s.telegram_chat_id.as_deref(),
                Some("keep"),
                "ningún campo debe importarse si el cron es inválido"
            );
        }
        let after_db = persisted_cron(&db_pool).await;
        assert_eq!(before_db, after_db, "no debe escribirse en la base");
        assert_eq!(
            after_db.as_deref(),
            Some("0 6 * * *"),
            "el cron previo persistido debe quedar intacto"
        );
    }

    /// validate-cron-on-put (ampliación): importar un settings con un cron de 6
    /// campos ⇒ 400, sin tocar memoria ni disco.
    #[tokio::test]
    async fn test_import_config_six_field_cron_rejected() {
        let db_pool = db::test_pool();
        seed_persisted_cron(&db_pool, "*/15 * * * *").await;
        let settings = Arc::new(Mutex::new(Settings::default()));
        let before_db = persisted_cron(&db_pool).await;

        let payload = ImportPayload {
            settings: Settings {
                update_check_cron: Some("0 0 6 * * *".into()),
                ..Default::default()
            },
        };

        let result = import_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(payload),
        )
        .await;
        match result {
            Err(AppError::BadRequest(_)) => {}
            other => panic!("se esperaba 400 (BadRequest), obtenido: {other:?}"),
        }

        {
            let s = settings.lock().await;
            assert_eq!(
                s.update_check_cron, None,
                "no debe mutar la memoria con 6 campos"
            );
        }
        let after_db = persisted_cron(&db_pool).await;
        assert_eq!(before_db, after_db, "no debe escribirse en la base");
        assert_eq!(
            after_db.as_deref(),
            Some("*/15 * * * *"),
            "el cron previo persistido debe quedar intacto"
        );
    }

    /// validate-cron-on-put (ampliación): importar un settings con un cron de
    /// solo espacios (`Some("   ")`) ⇒ 400, sin tocar memoria ni disco. Alinea
    /// el import con el PUT: un cron presente y no vacío debe ser válido.
    #[tokio::test]
    async fn test_import_config_whitespace_cron_rejected() {
        let db_pool = db::test_pool();
        seed_persisted_cron(&db_pool, "0 6 * * *").await;
        let settings = Arc::new(Mutex::new(Settings {
            update_check_cron: Some("0 6 * * *".into()),
            telegram_chat_id: Some("keep".into()),
            ..Default::default()
        }));
        let before_db = persisted_cron(&db_pool).await;

        let payload = ImportPayload {
            settings: Settings {
                update_check_cron: Some("   ".into()),
                telegram_chat_id: Some("imported".into()),
                ..Default::default()
            },
        };

        let result = import_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(payload),
        )
        .await;
        match result {
            Err(AppError::BadRequest(_)) => {}
            other => panic!("se esperaba 400 (BadRequest), obtenido: {other:?}"),
        }

        {
            let s = settings.lock().await;
            assert_eq!(
                s.update_check_cron.as_deref(),
                Some("0 6 * * *"),
                "el cron en memoria debe quedar intacto"
            );
            assert_eq!(
                s.telegram_chat_id.as_deref(),
                Some("keep"),
                "ningún campo debe importarse con un cron solo-espacios"
            );
        }
        let after_db = persisted_cron(&db_pool).await;
        assert_eq!(before_db, after_db, "no debe escribirse en la base");
        assert_eq!(
            after_db.as_deref(),
            Some("0 6 * * *"),
            "el cron previo persistido debe quedar intacto"
        );
    }

    /// validate-cron-on-put (ampliación): importar un settings sin cron (None)
    /// ⇒ 200 y el settings se aplica (memoria + disco).
    #[tokio::test]
    async fn test_import_config_none_cron_accepted() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings {
                update_check_cron: None,
                telegram_chat_id: Some("imported".into()),
                ..Default::default()
            },
        };

        let result = import_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(payload),
        )
        .await
        .expect("un settings sin cron debe aceptarse");
        assert_eq!(result.0["status"], "imported");

        {
            let s = settings.lock().await;
            assert_eq!(s.update_check_cron, None);
            assert_eq!(s.telegram_chat_id.as_deref(), Some("imported"));
        }
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        let loaded = db::load_settings(&guard).unwrap();
        assert_eq!(loaded.update_check_cron, None);
        assert_eq!(loaded.telegram_chat_id.as_deref(), Some("imported"));
    }

    /// validate-cron-on-put (ampliación): importar un settings con cron vacío
    /// (`Some("")`) ⇒ 200 y se aplica (normalizado a `None` en disco).
    #[tokio::test]
    async fn test_import_config_empty_cron_accepted() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings::default()));

        let payload = ImportPayload {
            settings: Settings {
                update_check_cron: Some(String::new()),
                telegram_chat_id: Some("imported".into()),
                ..Default::default()
            },
        };

        let result = import_config_h(
            State(settings.clone()),
            State(db_pool.clone()),
            Json(payload),
        )
        .await
        .expect("un cron vacío debe aceptarse (sin override)");
        assert_eq!(result.0["status"], "imported");

        {
            let s = settings.lock().await;
            assert_eq!(
                s.update_check_cron,
                Some(String::new()),
                "el cron vacío queda como Some(\"\") en memoria (sin normalizar)"
            );
            assert_eq!(s.telegram_chat_id.as_deref(), Some("imported"));
        }
        let conn = db_pool.get().await.unwrap();
        let guard = conn.lock().unwrap();
        let loaded = db::load_settings(&guard).unwrap();
        assert_eq!(
            loaded.update_check_cron, None,
            "el cron vacío se normaliza a None en disco"
        );
    }
}
