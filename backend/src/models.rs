use serde::de::{Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_tag: String,
    pub size_mb: f64,
    pub status: String,
    pub state: String,
    pub has_update: bool,
    pub updating: bool,
    pub compose_project: Option<String>,
    pub ports: Vec<String>,
    pub traefik_url: Option<String>,
    pub registry_url: String,
    #[serde(default)]
    pub last_check: Option<String>,
    #[serde(default)]
    pub next_check: Option<String>,
    #[serde(default)]
    pub last_remote_digest: String, // stores last verified remote config digest
    /// Config digest of the last image pulled into the local cache. Independent
    /// from `last_remote_digest` (which is the image the container runs).
    #[serde(default)]
    pub last_pulled_digest: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateEvent {
    pub containers: Vec<ContainerInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct UpdateProgress {
    pub container: String,
    pub status: String,
    pub done: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub checked: u32,
    #[serde(default)]
    pub updated: u32,
    #[serde(default)]
    pub errors: u32,
    #[serde(default)]
    pub updatable: u32,
    #[serde(default)]
    pub uptodate: u32,
    #[serde(default)]
    pub failed: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct NotifEvent {
    pub container: String,
    pub status: String,
    pub timestamp: String,
}

/// Incremental container state-change payload for the `container-event` SSE
/// message of `GET /api/stream`.
#[derive(Clone, Debug, Serialize)]
pub struct ContainerEvent {
    pub name: String,
    /// Normalized Docker action inferred from the state transition
    /// (`start|stop|die|pause|unpause|restart|create|destroy|update`).
    pub action: String,
    pub state: String,
    pub timestamp: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicConfig {
    pub oidc_configured: bool,
    pub port: u16,
    pub timezone: String,
    pub telegram_configured: bool,
    pub matrix_configured: bool,
    pub webhook_configured: bool,
    pub valet_configured: bool,
    /// Whether a Telegram token is stored server-side (value never exposed).
    pub telegram_token_set: bool,
    /// Whether a Matrix token is stored server-side (value never exposed).
    pub matrix_token_set: bool,
    /// Whether a Valet token is stored server-side (value never exposed).
    pub valet_token_set: bool,
    pub telegram_chat_id: Option<String>,
    /// Telegram forum topic (message thread) the notifications are routed to.
    /// Not a secret, exposed verbatim so the UI can preload the field.
    pub telegram_topic_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_room: Option<String>,
    pub valet_url: Option<String>,
    pub version: String,
    pub build_date: String,
    pub repo_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,
    pub name: String,
    pub email: String,
    pub exp: usize,
    /// Issued at (epoch seconds) — when the session was created.
    /// Used to enforce max session duration.
    pub iat: usize,
    /// Last activity timestamp (epoch seconds).
    /// Used to enforce idle timeout.
    pub last_active: usize,
    /// Optional purpose marker. Session tokens MUST NOT set it; SSE tickets
    /// are represented by [`SseTicketClaims`], not this type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// Claims for an ephemeral SSE ticket.
///
/// Tickets are signed with the same secret as session tokens but carry
/// `purpose = "sse"` and a short TTL (60 s). They are only accepted by the
/// auth middleware as the `token` query parameter on `/api/stream`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseTicketClaims {
    pub sub: String,
    /// Always `"sse"` for tickets issued by `/api/auth/sse-token`.
    pub purpose: String,
    pub iat: usize,
    pub exp: usize,
}

/// JSON body returned by `GET /api/auth/sse-token`.
#[derive(Debug, Clone, Serialize)]
pub struct SseTicketResponse {
    pub token: String,
    pub expires_in: u64,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub exp: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContainerInspectResponse {
    pub id: String,
    pub name: String,
    pub image: String,
    pub created: String,
    pub state: String,
    pub status: String,
    pub ports: Vec<PortInfo>,
    pub mounts: Vec<MountInfo>,
    pub env: Vec<String>,
    pub networks: Vec<ContainerNetworkInfo>,
    pub labels: HashMap<String, String>,
    pub restart_policy: String,
    pub health: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortInfo {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub r#type: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MountInfo {
    pub source: String,
    pub destination: String,
    pub mode: String,
    pub rw: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContainerNetworkInfo {
    pub name: String,
    pub ip_address: String,
    pub gateway: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackService {
    pub service: String,
    pub container_name: String,
    pub image: String,
    pub status: String,
    pub state: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackInfo {
    pub project: String,
    pub services: Vec<StackService>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackUpdateResult {
    pub service: String,
    pub status: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackUpdateResponse {
    pub project: String,
    pub results: Vec<StackUpdateResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateHistoryEntry {
    pub container: String,
    pub image: String,
    pub old_digest: String,
    pub new_digest: String,
    pub timestamp: String,
    pub status: String,
    pub duration_ms: u64,
    /// Nº de intentos de arranque sano realizados por la guarda de actualización.
    /// 0 cuando la entrada no pasa por la guarda (entradas antiguas o acciones `pull`).
    #[serde(default)]
    pub attempts: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub telegram_token: Option<String>,
    #[serde(default)]
    pub telegram_chat_id: Option<String>,
    #[serde(default)]
    pub telegram_topic_id: Option<String>,
    #[serde(default)]
    pub matrix_homeserver: Option<String>,
    #[serde(default)]
    pub matrix_token: Option<String>,
    #[serde(default)]
    pub matrix_room: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub valet_url: Option<String>,
    #[serde(default)]
    pub valet_token: Option<String>,

    #[serde(default)]
    pub update_check_cron: Option<String>,
    #[serde(default)]
    pub update_check_enabled: Option<bool>,
    #[serde(default)]
    pub update_check_notify: Option<bool>,
    #[serde(default)]
    pub default_update_action: Option<String>,
    #[serde(default)]
    pub default_cleanup_old_image: Option<bool>,
    #[serde(default)]
    pub default_rollback_on_failure: Option<bool>,

    #[serde(default)]
    pub update_check_last_run_at: Option<String>,

    /// Timeout en segundos para el pull de imágenes. Default: 600 (10 min).
    /// Aumentar para imágenes muy grandes (>500MB) o conexiones lentas.
    #[serde(default)]
    pub pull_timeout_secs: Option<u64>,

    /// Intervalo en milisegundos entre verificaciones de digest de cada contenedor
    /// en check_all_h y update_check_worker. Default: 2000 (2s).
    /// Aumentar para evitar rate limiting en registries públicos.
    #[serde(default)]
    pub check_interval_ms: Option<u64>,

    /// Nº máximo de intentos de arranque sano antes de hacer rollback. Default: 3.
    #[serde(default = "default_update_max_start_attempts")]
    pub update_max_start_attempts: u32,

    /// Segundos de espera entre intentos de arranque. Default: 10.
    #[serde(default = "default_update_start_attempt_interval_secs")]
    pub update_start_attempt_interval_secs: u64,

    /// Segundos de ventana estable para considerar sano un arranque. Default: 30.
    #[serde(default = "default_update_health_window_secs")]
    pub update_health_window_secs: u64,

    /// Override de la **política por defecto** del nº máximo de intentos de
    /// arranque sano. `None` = sin override (aplica solo al *fallback* de
    /// contenedores sin política específica).
    #[serde(default)]
    pub default_max_start_attempts: Option<u32>,

    /// Override de la **política por defecto** de la ventana estable de salud
    /// (segundos). `None` = sin override.
    #[serde(default)]
    pub default_health_window_secs: Option<u64>,

    /// Días de retención del histórico de actualizaciones (`update_history`).
    /// `None` → se aplica el default (30). `0` → retención ilimitada.
    #[serde(default)]
    pub update_history_retention_days: Option<u32>,
}

/// Default global: nº máximo de intentos de arranque sano antes del rollback.
pub const DEFAULT_UPDATE_MAX_START_ATTEMPTS: u32 = 3;
/// Default global: segundos de espera entre intentos de arranque.
pub const DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS: u64 = 10;
/// Default global: segundos de ventana estable para considerar un arranque sano.
pub const DEFAULT_UPDATE_HEALTH_WINDOW_SECS: u64 = 30;

/// Único origen del cron del auto-update. Default: todos los días a las 06:00.
pub const DEFAULT_UPDATE_CHECK_CRON: &str = "0 6 * * *";
/// Único origen del timeout (segundos) para el pull de imágenes.
pub const DEFAULT_PULL_TIMEOUT_SECS: u64 = 600;
/// Único origen del intervalo (ms) entre verificaciones de digest de cada contenedor.
pub const DEFAULT_CHECK_INTERVAL_MS: u64 = 2000;

/// Defaults canónicos de la configuración de updates, expuestos por
/// `GET /api/update-check/defaults` para que el frontend deje de duplicar los
/// literales. Se pueblan siempre desde las consts de este módulo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDefaults {
    pub cron: String,
    pub pull_timeout_secs: u64,
    pub check_interval_ms: u64,
    pub update_max_start_attempts: u32,
    pub update_start_attempt_interval_secs: u64,
    pub update_health_window_secs: u64,
}

impl UpdateDefaults {
    /// Construye los defaults canónicos desde las consts (`DEFAULT_UPDATE_CHECK_CRON`,
    /// `DEFAULT_PULL_TIMEOUT_SECS`, `DEFAULT_CHECK_INTERVAL_MS`,
    /// `DEFAULT_UPDATE_MAX_START_ATTEMPTS`, `DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS`,
    /// `DEFAULT_UPDATE_HEALTH_WINDOW_SECS`). Sin literales.
    pub fn canonical() -> Self {
        Self {
            cron: DEFAULT_UPDATE_CHECK_CRON.to_string(),
            pull_timeout_secs: DEFAULT_PULL_TIMEOUT_SECS,
            check_interval_ms: DEFAULT_CHECK_INTERVAL_MS,
            update_max_start_attempts: DEFAULT_UPDATE_MAX_START_ATTEMPTS,
            update_start_attempt_interval_secs: DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS,
            update_health_window_secs: DEFAULT_UPDATE_HEALTH_WINDOW_SECS,
        }
    }
}

pub(crate) const fn default_update_max_start_attempts() -> u32 {
    DEFAULT_UPDATE_MAX_START_ATTEMPTS
}

pub(crate) const fn default_update_start_attempt_interval_secs() -> u64 {
    DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS
}

pub(crate) const fn default_update_health_window_secs() -> u64 {
    DEFAULT_UPDATE_HEALTH_WINDOW_SECS
}

/// Configuración de retención de histórico de actualizaciones.
///
/// `retention_days: 0` significa retención ilimitada (no se poda).
/// Default: 30 días.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateHistoryRetention {
    /// Días de retención. `0` significa retención ilimitada.
    pub retention_days: u32,
}

impl Default for UpdateHistoryRetention {
    fn default() -> Self {
        Self { retention_days: 30 }
    }
}

/// Resultado de una operación de poda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneResult {
    /// Número de filas eliminadas.
    pub deleted: u64,
    /// Días de retención aplicados (0 = ilimitada, deleted siempre 0).
    pub retention_days: u32,
    /// Duración de la operación en milisegundos.
    pub duration_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telegram_token: None,
            telegram_chat_id: None,
            telegram_topic_id: None,
            matrix_homeserver: None,
            matrix_token: None,
            matrix_room: None,
            webhook_url: None,
            valet_url: None,
            valet_token: None,
            update_check_cron: None,
            update_check_enabled: None,
            update_check_notify: None,
            default_update_action: None,
            default_cleanup_old_image: None,
            default_rollback_on_failure: None,
            update_check_last_run_at: None,
            pull_timeout_secs: None,
            check_interval_ms: None,
            update_max_start_attempts: DEFAULT_UPDATE_MAX_START_ATTEMPTS,
            update_start_attempt_interval_secs: DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS,
            update_health_window_secs: DEFAULT_UPDATE_HEALTH_WINDOW_SECS,
            default_max_start_attempts: None,
            default_health_window_secs: None,
            update_history_retention_days: None,
        }
    }
}

impl Settings {
    /// Cron efectivo del auto-update. Aplica el default canónico cuando el
    /// valor almacenado es `None` **o vacío** (misma semántica que la
    /// normalización de `db.rs`); todos los puntos de uso deben llamar aquí
    /// en lugar de repetir el literal.
    pub fn effective_update_check_cron(&self) -> String {
        self.update_check_cron
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_UPDATE_CHECK_CRON.to_string())
    }

    /// Timeout efectivo (segundos) para el pull de imágenes, con fallback a
    /// `DEFAULT_PULL_TIMEOUT_SECS` cuando el valor almacenado es `None`.
    pub fn effective_pull_timeout_secs(&self) -> u64 {
        self.pull_timeout_secs.unwrap_or(DEFAULT_PULL_TIMEOUT_SECS)
    }

    /// Intervalo efectivo (ms) entre verificaciones de digest, con fallback a
    /// `DEFAULT_CHECK_INTERVAL_MS` cuando el valor almacenado es `None`.
    pub fn effective_check_interval_ms(&self) -> u64 {
        self.check_interval_ms.unwrap_or(DEFAULT_CHECK_INTERVAL_MS)
    }

    /// Overrides de la guarda de la política por defecto (aplican al fallback de
    /// contenedores sin política específica).
    pub fn default_guard_overrides(&self) -> (Option<u32>, Option<u64>) {
        (
            self.default_max_start_attempts,
            self.default_health_window_secs,
        )
    }

    /// Construye la `UpdatePolicy` de *fallback* para un contenedor **sin**
    /// política específica: acción/limpieza/rollback de la política por defecto
    /// y los overrides de la guarda por defecto (`None` si no hay).
    /// `notify_events: false`.
    pub fn fallback_update_policy(&self, container: impl Into<String>) -> UpdatePolicy {
        UpdatePolicy {
            container: container.into(),
            action: self
                .default_update_action
                .clone()
                .unwrap_or_else(|| "pull-restart".into())
                .parse()
                .unwrap_or(UpdateAction::PullRestart),
            cleanup_old_image: self.default_cleanup_old_image.unwrap_or(false),
            rollback_on_failure: self.default_rollback_on_failure.unwrap_or(false),
            notify_events: false,
            max_start_attempts: self.default_max_start_attempts,
            health_window_secs: self.default_health_window_secs,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePolicy {
    pub container: String,
    pub action: UpdateAction,
    pub cleanup_old_image: bool,
    pub rollback_on_failure: bool,
    #[serde(default = "default_notify_events")]
    pub notify_events: bool,
    /// Override por política del nº máximo de intentos de arranque sano.
    #[serde(default)]
    pub max_start_attempts: Option<u32>,
    /// Override por política de la ventana estable de salud (segundos).
    #[serde(default)]
    pub health_window_secs: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePolicyReq {
    pub action: UpdateAction,
    pub cleanup_old_image: bool,
    pub rollback_on_failure: bool,
    #[serde(default = "default_notify_events")]
    pub notify_events: bool,
    /// Override por política del nº máximo de intentos de arranque sano.
    #[serde(default)]
    pub max_start_attempts: Option<u32>,
    /// Override por política de la ventana estable de salud (segundos).
    #[serde(default)]
    pub health_window_secs: Option<u64>,
}

const fn default_notify_events() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateAction {
    None,
    Pull,
    PullRestart,
    PullRestartStack,
}

impl std::fmt::Display for UpdateAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateAction::None => write!(f, "none"),
            UpdateAction::Pull => write!(f, "pull"),
            UpdateAction::PullRestart => write!(f, "pull-restart"),
            UpdateAction::PullRestartStack => write!(f, "pull-restart-stack"),
        }
    }
}

impl std::str::FromStr for UpdateAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(UpdateAction::None),
            "pull" => Ok(UpdateAction::Pull),
            "pull-restart" => Ok(UpdateAction::PullRestart),
            "pull-restart-stack" => Ok(UpdateAction::PullRestartStack),
            _ => Err(format!("unknown action: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateCheckConfig {
    pub cron: String,
    pub enabled: bool,
    pub notify: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    /// Timeout en segundos para pull de imágenes. Default: 600 (10 min).
    #[serde(default)]
    pub pull_timeout_secs: Option<u64>,
    /// Intervalo en ms entre verificaciones de cada contenedor.
    #[serde(default)]
    pub check_interval_ms: Option<u64>,
    /// Nº máximo de intentos de arranque sano antes del rollback. Default: 3.
    #[serde(default = "default_update_max_start_attempts")]
    pub update_max_start_attempts: u32,
    /// Segundos de espera entre intentos de arranque. Default: 10.
    #[serde(default = "default_update_start_attempt_interval_secs")]
    pub update_start_attempt_interval_secs: u64,
    /// Segundos de ventana estable de salud. Default: 30.
    #[serde(default = "default_update_health_window_secs")]
    pub update_health_window_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateSettingsReq {
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub telegram_topic_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_token: Option<String>,
    pub matrix_room: Option<String>,
    pub webhook_url: Option<String>,
    pub valet_url: Option<String>,
    pub valet_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TestNotificationReq {
    pub channel: String, // "telegram" or "matrix"
}

#[derive(Clone, Debug, Serialize)]
pub struct VersionCompare {
    pub local_tag: String,
    pub remote_tag: Option<String>,
    pub has_update: Option<bool>,
    pub local_digest: Option<String>,
    pub remote_digest: Option<String>,
    pub changelog_url: Option<String>,
    pub error: Option<String>,
}

#[allow(dead_code)]
pub fn default_enabled() -> bool {
    true
}

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

// ── Constants ──────────────────────────────────────────────

#[expect(dead_code)]
pub const ALL_CONTAINERS: &str = "*";

pub const LABEL_COMPOSE_PROJECT: &str = "com.docker.compose.project";
pub const LABEL_COMPOSE_SERVICE: &str = "com.docker.compose.service";

pub fn strip_name(name: &str) -> String {
    name.trim_start_matches('/').to_string()
}

/// Extract the tag portion from a full image reference.
/// Returns `("paradedb/paradedb", "latest")` for `"paradedb/paradedb:latest"`.
/// Returns `("alpine", "latest")` for `"alpine"` (no tag, defaults to latest).
#[allow(dead_code)]
pub fn parse_image_tag(image: &str) -> (String, String) {
    if let Some(pos) = image.rfind('@') {
        (image[..pos].to_string(), "digest".to_string())
    } else if let Some(pos) = image.rfind(':') {
        // Ensure we don't split on port numbers like registry:5000/image
        // Only split if the part after ':' looks like a tag (no '/')
        let after = &image[pos + 1..];
        if after.contains('/') {
            // This is a registry with port, e.g. registry:5000/image
            (image.to_string(), "latest".to_string())
        } else {
            (image[..pos].to_string(), image[pos + 1..].to_string())
        }
    } else {
        (image.to_string(), "latest".to_string())
    }
}

/// Return the current platform string in Docker format: `os/arch`.
/// Maps Rust architecture names to Docker convention.
#[allow(dead_code)]
pub fn current_platform() -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "arm",
        "riscv64" => "riscv64",
        other => other,
    };
    format!("{}/{}", std::env::consts::OS, arch)
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("Docker: {0}")]
    Docker(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    /// Traduce un fallo de infraestructura de datos (pool agotada/cerrada o
    /// `Mutex` de conexión envenenado) a un `AppError::Internal` (HTTP 500).
    ///
    /// Centraliza la conversión para que los handlers no paniquen con
    /// `unwrap()`: `db_pool.get().await.map_err(AppError::from_pool)?`.
    /// Se usa un helper genérico en lugar de un `impl From<PoolError>` para no
    /// acoplar `models.rs` al tipo concreto de pool.
    pub fn from_pool<E: std::fmt::Display>(e: E) -> Self {
        AppError::Internal(format!("base de datos no disponible: {e}"))
    }

    /// Traduce un fallo de una consulta a SQLite (`rusqlite`) a
    /// `AppError::Internal` (HTTP 500).
    ///
    /// Compañero de [`AppError::from_pool`] para que los handlers no repitan
    /// `AppError::Internal(format!(...))` por cada `query_row`/`prepare`:
    /// `db::load_update_history_page_with_total(...).map_err(AppError::from_db)?`.
    pub fn from_db<E: std::fmt::Display>(e: E) -> Self {
        AppError::Internal(format!("consulta a base de datos fallida: {e}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Docker(m) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Docker: {}", m)),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        tracing::error!("{:?}: {}", status, msg);
        (status, Json(serde_json::json!({"error": msg}))).into_response()
    }
}

impl From<StatusCode> for AppError {
    fn from(s: StatusCode) -> Self {
        AppError::Internal(s.to_string())
    }
}

impl From<bollard::errors::Error> for AppError {
    fn from(e: bollard::errors::Error) -> Self {
        AppError::Docker(e.to_string())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ContainerSummary {
    pub total: usize,
    pub running: usize,
    pub stopped: usize,
    pub paused: usize,
    pub with_updates: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BatchProgress {
    pub total: u32,
    pub checked: u32,
    pub updated: u32,
    pub errors: u32,
    pub checking: String,
    #[serde(default)]
    pub updatable: u32,
    #[serde(default)]
    pub uptodate: u32,
    #[serde(default)]
    pub failed: u32,
}

/// Tamaños de página aceptados por los endpoints de listado.
pub const ALLOWED_PAGE_SIZES: [u32; 3] = [10, 20, 50];
/// Página por defecto (`page`).
pub const DEFAULT_PAGE: u32 = 1;
/// Tamaño de página por defecto (`page_size`).
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// Parámetros de listado tal y como llegan por query string.
///
/// Todos los campos son opcionales para poder distinguir "ausente" de "valor por
/// defecto"; la validación y normalización se hace con
/// [`ContainerListQuery::normalize`].
///
/// La deserialización es manual (ver el `impl Deserialize` más abajo) porque
/// `serde_urlencoded` —el deserializador que usa `axum::extract::Query`— no sabe
/// mapear claves repetidas a un campo escalar. Tolerancias documentadas en ese
/// `impl`: escalares repetidos → *last-wins*; claves desconocidas → ignoradas.
#[derive(Clone, Debug, Default)]
pub struct ContainerListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// Substring case-insensitive sobre `name`, `image` y `compose_project`.
    pub q: Option<String>,
    /// Estados aceptados. Puede repetirse (`?state=running&state=exited`) y se
    /// aplica con semántica OR; un conjunto vacío no filtra por estado.
    pub state: Vec<String>,
    /// Si es `true`, limita a contenedores con actualización pendiente.
    pub has_update: Option<bool>,
}

/// Deserialización manual de [`ContainerListQuery`].
///
/// **Por qué no `derive`**: `axum::extract::Query` usa `serde_urlencoded`, que
/// deserializa el top-level como un mapa de pares clave/valor. Con `derive`, un
/// campo `Vec<String>` recibe el valor de la **primera** ocurrencia como un
/// escalar y falla (`invalid type: string "running", expected a sequence`), y un
/// campo escalar rechaza la segunda ocurrencia (`duplicate field`). Comprobado
/// empíricamente para `Vec<String>`, `Option<Vec<String>>` y `Vec<(String, String)>`.
///
/// Este `visit_map` recorre las entradas en orden —incluidas las claves
/// repetidas— y acumula todos los valores de `state` en el `Vec`.
///
/// # Tolerancia deliberada (difiere del `derive` de serde)
///
/// - **Claves escalares repetidas** (`?page=1&page=2`): **last-wins**. La última
///   ocurrencia sobrescribe a la anterior en vez de provocar un error
///   (`derive` respondería `400 duplicate field`). Es el comportamiento natural
///   de una query string y evita romper a clientes que añaden parámetros.
/// - **Claves desconocidas** (`?pageSize=50`): se **ignoran** a propósito
///   (tolerancia hacia delante, para añadir parámetros sin romper clientes
///   antiguos). Consecuencia asumida: un *typo* cae al valor por defecto en vez
///   de devolver `400`.
///
/// Los valores con el tipo equivocado (`?page=abc`, `?has_update=maybe`,
/// `?page_size=-1`, `?page=4294967296`) sí son un error de deserialización y
/// producen `400`.
impl<'de> Deserialize<'de> for ContainerListQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContainerListQueryVisitor;

        impl<'de> Visitor<'de> for ContainerListQueryVisitor {
            type Value = ContainerListQuery;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("una query string con page, page_size, q, state y has_update")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut query = ContainerListQuery::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "page" => query.page = Some(map.next_value()?),
                        "page_size" => query.page_size = Some(map.next_value()?),
                        "q" => query.q = Some(map.next_value()?),
                        // Puede aparecer varias veces: se acumulan los valores.
                        "state" => query.state.push(map.next_value()?),
                        "has_update" => query.has_update = Some(map.next_value()?),
                        // Clave desconocida: se ignora (tolerancia hacia delante).
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(query)
            }
        }

        deserializer.deserialize_map(ContainerListQueryVisitor)
    }
}

impl ContainerListQuery {
    /// Valida y normaliza la query a [`ContainerListParams`].
    ///
    /// Aplica los defaults (`page = 1`, `page_size = 20`) y recorta los filtros
    /// de texto en blanco (`q`/`state` vacíos se tratan como ausentes).
    ///
    /// # Errors
    ///
    /// Devuelve [`AppError::BadRequest`] (HTTP 400) si `page` es 0 o si
    /// `page_size` no pertenece a [`ALLOWED_PAGE_SIZES`].
    pub fn normalize(self) -> Result<ContainerListParams, AppError> {
        let page = self.page.unwrap_or(DEFAULT_PAGE);
        if page == 0 {
            return Err(AppError::BadRequest(format!(
                "page debe ser >= 1 (recibido {page})"
            )));
        }

        let page_size = self.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        if !ALLOWED_PAGE_SIZES.contains(&page_size) {
            return Err(AppError::BadRequest(format!(
                "page_size debe ser uno de {ALLOWED_PAGE_SIZES:?} (recibido {page_size})"
            )));
        }

        Ok(ContainerListParams {
            page,
            page_size,
            q: normalize_filter(self.q),
            state: normalize_state_values(self.state),
            has_update: self.has_update,
        })
    }
}

/// Filtros y paginación ya validados y normalizados.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerListParams {
    pub page: u32,
    pub page_size: u32,
    pub q: Option<String>,
    /// Conjunto de estados aceptados (semántica OR). Vacío → no filtra por estado.
    pub state: Vec<String>,
    pub has_update: Option<bool>,
}

impl Default for ContainerListParams {
    fn default() -> Self {
        Self {
            page: DEFAULT_PAGE,
            page_size: DEFAULT_PAGE_SIZE,
            q: None,
            state: Vec::new(),
            has_update: None,
        }
    }
}

/// Recorta un filtro de texto y descarta los que quedan vacíos.
fn normalize_filter(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Recorta cada estado, descarta los vacíos y deduplica conservando el orden.
fn normalize_state_values(values: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::with_capacity(values.len());
    for value in values {
        let value = value.trim();
        if value.is_empty() || normalized.iter().any(|kept| kept == value) {
            continue;
        }
        normalized.push(value.to_string());
    }
    normalized
}

/// Página de contenedores devuelta por los endpoints de listado.
#[derive(Clone, Debug, Serialize)]
pub struct PagedContainers {
    pub items: Vec<ContainerInfo>,
    /// Recuento **tras** aplicar los filtros (no de la página).
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

/// Parámetros de `GET /api/history` tal y como llegan por query string.
///
/// La deserialización es manual (ver el `impl Deserialize` más abajo) porque
/// `serde_urlencoded` —el deserializador que usa `axum::extract::Query`— no sabe
/// mapear claves repetidas a un campo escalar. Tolerancias idénticas a
/// [`ContainerListQuery`]: escalares repetidos → *last-wins*; claves
/// desconocidas → ignoradas; `status` acumula todas sus ocurrencias.
#[derive(Clone, Debug, Default)]
pub struct UpdateHistoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// Substring case-insensitive sobre `container`, `image`, `old_digest` y `new_digest`.
    pub q: Option<String>,
    /// Estados aceptados. Puede repetirse (`?status=error&status=rolled-back`) y
    /// se aplica con semántica OR; un conjunto vacío no filtra por estado.
    pub status: Vec<String>,
}

impl UpdateHistoryQuery {
    /// Valida y normaliza la query a [`UpdateHistoryParams`].
    ///
    /// Aplica los defaults (`page = 1`, `page_size = 20`) y recorta los filtros
    /// de texto en blanco (`q`/`status` vacíos se tratan como ausentes). Los
    /// valores no numéricos (`?page=abc`) ya fallan en la deserialización.
    ///
    /// # Errors
    ///
    /// Devuelve [`AppError::BadRequest`] (HTTP 400) si `page` es 0 o si
    /// `page_size` no pertenece a [`ALLOWED_PAGE_SIZES`].
    pub fn normalize(self) -> Result<UpdateHistoryParams, AppError> {
        let page = self.page.unwrap_or(DEFAULT_PAGE);
        if page == 0 {
            return Err(AppError::BadRequest(format!(
                "page debe ser >= 1 (recibido {page})"
            )));
        }

        let page_size = self.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
        if !ALLOWED_PAGE_SIZES.contains(&page_size) {
            return Err(AppError::BadRequest(format!(
                "page_size debe ser uno de {ALLOWED_PAGE_SIZES:?} (recibido {page_size})"
            )));
        }

        Ok(UpdateHistoryParams {
            page,
            page_size,
            q: normalize_filter(self.q),
            status: normalize_state_values(self.status),
        })
    }
}

/// Deserialización manual de [`UpdateHistoryQuery`].
///
/// **Por qué no `derive`**: `axum::extract::Query` usa `serde_urlencoded`, que
/// deserializa el top-level como un mapa de pares clave/valor. Con `derive`, un
/// campo `Vec<String>` recibe el valor de la **primera** ocurrencia como un
/// escalar y falla (`invalid type: string "error", expected a sequence`), y un
/// campo escalar rechaza la segunda ocurrencia (`duplicate field`).
///
/// Este `visit_map` recorre las entradas en orden —incluidas las claves
/// repetidas— y acumula todos los valores de `status` en el `Vec`, replicando
/// exactamente las tolerancias de [`ContainerListQuery`]:
///
/// - **Claves escalares repetidas** (`?page=1&page=2`): **last-wins**.
/// - **Claves desconocidas** (`?pageSize=50`): se **ignoran** a propósito.
///
/// Los valores con el tipo equivocado (`?page=abc`, `?page_size=-1`) sí son un
/// error de deserialización y producen `400`.
impl<'de> Deserialize<'de> for UpdateHistoryQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UpdateHistoryQueryVisitor;

        impl<'de> Visitor<'de> for UpdateHistoryQueryVisitor {
            type Value = UpdateHistoryQuery;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("una query string con page, page_size, q y status")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut query = UpdateHistoryQuery::default();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "page" => query.page = Some(map.next_value()?),
                        "page_size" => query.page_size = Some(map.next_value()?),
                        "q" => query.q = Some(map.next_value()?),
                        // Puede aparecer varias veces: se acumulan los valores.
                        "status" => query.status.push(map.next_value()?),
                        // Clave desconocida: se ignora (tolerancia hacia delante).
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(query)
            }
        }

        deserializer.deserialize_map(UpdateHistoryQueryVisitor)
    }
}

/// Filtros y paginación del histórico ya validados y normalizados.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateHistoryParams {
    pub page: u32,
    pub page_size: u32,
    pub q: Option<String>,
    /// Conjunto de estados aceptados (semántica OR). Vacío → no filtra por estado.
    pub status: Vec<String>,
}

impl Default for UpdateHistoryParams {
    fn default() -> Self {
        Self {
            page: DEFAULT_PAGE,
            page_size: DEFAULT_PAGE_SIZE,
            q: None,
            status: Vec::new(),
        }
    }
}

/// Página del histórico devuelta por `GET /api/history`.
#[derive(Clone, Debug, Serialize)]
pub struct PagedUpdateHistory {
    pub items: Vec<UpdateHistoryEntry>,
    /// Recuento **tras** aplicar los filtros (no de la página).
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

/// Offset de la primera entrada de `page` (base 0) para un `page_size` dado.
///
/// Usa aritmética saturante en ambos pasos: `page = u32::MAX` desbordaría `usize`
/// en plataformas de 32 bits. Con el offset saturado, el `skip` deja la página
/// vacía sin panickear (que es el comportamiento contratado para una página más
/// allá del final).
pub(crate) fn page_offset(page: u32, page_size: u32) -> usize {
    (page.saturating_sub(1) as usize).saturating_mul(page_size as usize)
}

/// Filtra, ordena y pagina `containers` según `params`.
///
/// El orden es determinista: `compose_project` ascendente (`None` se trata como
/// cadena vacía, por lo que va primero) y, dentro de cada proyecto, `name`
/// ascendente. `total` es el recuento **tras** filtrar; una `page` más allá del
/// final devuelve `items` vacío conservando el `total` real.
pub fn paginate_containers(
    containers: &[ContainerInfo],
    params: &ContainerListParams,
) -> PagedContainers {
    let mut filtered: Vec<&ContainerInfo> = containers
        .iter()
        .filter(|c| container_matches(c, params))
        .collect();
    filtered.sort_by(|a, b| {
        a.compose_project
            .as_deref()
            .unwrap_or("")
            .cmp(b.compose_project.as_deref().unwrap_or(""))
            .then_with(|| a.name.cmp(&b.name))
    });

    let total = filtered.len();
    let offset = page_offset(params.page, params.page_size);
    let items = filtered
        .into_iter()
        .skip(offset)
        .take(params.page_size as usize)
        .cloned()
        .collect();

    PagedContainers {
        items,
        total,
        page: params.page,
        page_size: params.page_size,
    }
}

/// `true` si `c` satisface los filtros de `params`.
///
/// `state` se evalúa como pertenencia al conjunto (OR); `has_update` es igualdad
/// booleana y `q` es un substring case-insensitive sobre `name`, `image` y
/// `compose_project`.
fn container_matches(c: &ContainerInfo, params: &ContainerListParams) -> bool {
    if !params.state.is_empty() && !params.state.iter().any(|state| state == &c.state) {
        return false;
    }

    if let Some(has_update) = params.has_update {
        if c.has_update != has_update {
            return false;
        }
    }

    if let Some(q) = &params.q {
        let q = q.to_lowercase();
        let in_name = c.name.to_lowercase().contains(&q);
        let in_image = c.image.to_lowercase().contains(&q);
        let in_project = c
            .compose_project
            .as_deref()
            .is_some_and(|p| p.to_lowercase().contains(&q));
        if !(in_name || in_image || in_project) {
            return false;
        }
    }

    true
}

/// Construye la respuesta del listado/tiempo real para una flota completa.
///
/// El `summary` se calcula sobre **toda** la flota (independientemente de
/// `params`), mientras que `containers` es solo la página filtrada solicitada.
pub fn build_state_response(
    containers: Vec<ContainerInfo>,
    progress: BatchProgress,
    params: &ContainerListParams,
) -> StateResponse {
    let summary = compute_summary(&containers);
    let paged = paginate_containers(&containers, params);
    StateResponse {
        containers: paged.items,
        summary,
        progress,
        page: paged.page,
        page_size: paged.page_size,
        total: paged.total,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StateResponse {
    pub containers: Vec<ContainerInfo>,
    pub summary: ContainerSummary,
    #[serde(default)]
    pub progress: BatchProgress,
    /// Página efectiva entregada en `containers`.
    pub page: u32,
    /// Tamaño de página efectivo.
    pub page_size: u32,
    /// Número total de contenedores tras aplicar los filtros (no de la página).
    pub total: usize,
}

pub fn compute_summary(containers: &[ContainerInfo]) -> ContainerSummary {
    let total = containers.len();
    let running = containers.iter().filter(|c| c.state == "running").count();
    let stopped = containers
        .iter()
        .filter(|c| c.state != "running" && c.state != "paused")
        .count();
    let paused = containers.iter().filter(|c| c.state == "paused").count();
    let with_updates = containers.iter().filter(|c| c.has_update).count();
    ContainerSummary {
        total,
        running,
        stopped,
        paused,
        with_updates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Query;
    use axum::http::StatusCode;

    // ── helpers ─────────────────────────────────────────────

    fn make_container(name: &str, state: &str, has_update: bool) -> ContainerInfo {
        ContainerInfo {
            id: name.to_string(),
            name: name.to_string(),
            image: format!("{}:latest", name),
            image_tag: "latest".into(),
            size_mb: 10.0,
            status: state.to_string(),
            state: state.to_string(),
            has_update,
            compose_project: None,
            ports: vec![],
            traefik_url: None,
            updating: false,
            registry_url: String::new(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }
    }

    #[test]
    fn test_strip_name_with_slash() {
        assert_eq!(strip_name("/test"), "test");
    }

    #[test]
    fn test_strip_name_without_slash() {
        assert_eq!(strip_name("test"), "test");
    }

    #[test]
    fn test_strip_name_empty() {
        assert_eq!(strip_name(""), "");
    }

    #[test]
    fn test_strip_name_only_slash() {
        assert_eq!(strip_name("/"), "");
    }

    #[test]
    fn test_strip_name_multi_slash() {
        assert_eq!(strip_name("///test"), "test");
    }

    #[test]
    fn test_default_enabled() {
        assert!(default_enabled());
    }

    #[test]
    fn test_app_error_not_found_status() {
        let err = AppError::NotFound("missing".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_app_error_docker_status() {
        let err = AppError::Docker("connection failed".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_app_error_internal_status() {
        let err = AppError::Internal("oops".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_app_error_from_status_code() {
        let err = AppError::from(StatusCode::BAD_REQUEST);
        assert!(matches!(err, AppError::Internal(_)));
    }

    // ── parse_image_tag ────────────────────────────────────

    #[test]
    fn test_parse_image_tag_with_tag() {
        let (repo, tag) = parse_image_tag("nginx:latest");
        assert_eq!(repo, "nginx");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_parse_image_tag_with_version_tag() {
        let (repo, tag) = parse_image_tag("library/postgres:15-alpine");
        assert_eq!(repo, "library/postgres");
        assert_eq!(tag, "15-alpine");
    }

    #[test]
    fn test_parse_image_tag_with_digest() {
        let (repo, tag) = parse_image_tag(
            "nginx@sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert_eq!(repo, "nginx");
        assert_eq!(tag, "digest");
    }

    #[test]
    fn test_parse_image_tag_without_tag_defaults_latest() {
        let (repo, tag) = parse_image_tag("alpine");
        assert_eq!(repo, "alpine");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_parse_image_tag_with_registry_port() {
        // registry:5000/image — the colon is part of the registry URL, not a tag
        let (repo, tag) = parse_image_tag("registry.example.com:5000/myimage:v2");
        assert_eq!(repo, "registry.example.com:5000/myimage");
        assert_eq!(tag, "v2");
    }

    #[test]
    fn test_parse_image_tag_docker_io_with_tag() {
        let (repo, tag) = parse_image_tag("docker.io/library/redis:7.2");
        assert_eq!(repo, "docker.io/library/redis");
        assert_eq!(tag, "7.2");
    }

    #[test]
    fn test_parse_image_tag_empty() {
        let (repo, tag) = parse_image_tag("");
        assert_eq!(repo, "");
        assert_eq!(tag, "latest");
    }

    // ── current_platform ────────────────────────────────────

    #[test]
    fn test_current_platform_format() {
        let platform = current_platform();
        // Should be in format "os/arch" like "linux/amd64"
        assert!(
            platform.contains('/'),
            "Platform should contain '/': {}",
            platform
        );
        let parts: Vec<&str> = platform.split('/').collect();
        assert_eq!(parts.len(), 2, "Platform should have 2 parts: {}", platform);
    }

    // ── compute_summary / ContainerSummary / StateResponse ──
    //
    // NOTA: Estos tests NO compilarán hasta que se implementen:
    //   - ContainerSummary struct
    //   - StateResponse struct
    //   - compute_summary() fn
    // en models.rs. Eso es intencional (RED phase de TDD).

    #[test]
    fn test_compute_summary_all_running() {
        let containers = vec![
            make_container("web", "running", false),
            make_container("db", "running", false),
            make_container("cache", "running", false),
        ];
        let summary = compute_summary(&containers);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.running, 3);
        assert_eq!(summary.stopped, 0);
        assert_eq!(summary.paused, 0);
        assert_eq!(summary.with_updates, 0);
    }

    #[test]
    fn test_compute_summary_mixed_states() {
        let containers = vec![
            make_container("web-1", "running", false),
            make_container("web-2", "running", false),
            make_container("web-3", "running", false),
            make_container("web-4", "running", false),
            make_container("db-1", "exited", false),
            make_container("db-2", "exited", false),
            make_container("worker", "paused", false),
        ];
        let summary = compute_summary(&containers);
        assert_eq!(summary.total, 7);
        assert_eq!(summary.running, 4);
        assert_eq!(summary.stopped, 2);
        assert_eq!(summary.paused, 1);
        assert_eq!(summary.with_updates, 0);
    }

    #[test]
    fn test_compute_summary_with_updates() {
        let containers = vec![
            make_container("nginx", "running", true),
            make_container("redis", "running", true),
            make_container("postgres", "running", false),
            make_container("mysql", "exited", true),
            make_container("mongo", "exited", false),
        ];
        let summary = compute_summary(&containers);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.running, 3);
        assert_eq!(summary.stopped, 2);
        assert_eq!(summary.paused, 0);
        assert_eq!(summary.with_updates, 3);
    }

    #[test]
    fn test_compute_summary_empty() {
        let containers: Vec<ContainerInfo> = vec![];
        let summary = compute_summary(&containers);
        assert_eq!(summary.total, 0);
        assert_eq!(summary.running, 0);
        assert_eq!(summary.stopped, 0);
        assert_eq!(summary.paused, 0);
        assert_eq!(summary.with_updates, 0);
    }

    #[test]
    fn test_state_response_serialization() {
        let containers = vec![make_container("web", "running", false)];
        let summary = ContainerSummary {
            total: 1,
            running: 1,
            stopped: 0,
            paused: 0,
            with_updates: 0,
        };
        let progress = BatchProgress {
            total: 1,
            checked: 0,
            updated: 0,
            errors: 0,
            checking: "".into(),
            ..Default::default()
        };
        let response = StateResponse {
            containers,
            summary,
            progress,
            page: 1,
            page_size: 20,
            total: 1,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(
            json.get("containers").is_some(),
            "JSON must contain 'containers' field"
        );
        assert!(
            json.get("summary").is_some(),
            "JSON must contain 'summary' field"
        );
        assert!(
            json.get("progress").is_some(),
            "JSON must contain 'progress' field"
        );
        assert!(json.get("page").is_some(), "JSON must contain 'page' field");
        assert!(
            json.get("page_size").is_some(),
            "JSON must contain 'page_size' field"
        );
        assert!(
            json.get("total").is_some(),
            "JSON must contain 'total' field"
        );
    }

    // ── ContainerEvent (sse-first) ───────────────────────────

    /// `ContainerEvent` is the payload of the `container-event` SSE message:
    /// it MUST serialize with the four contract fields (task 1.1).
    #[test]
    fn test_container_event_serialization() {
        let evt = ContainerEvent {
            name: "nginx".into(),
            action: "start".into(),
            state: "running".into(),
            timestamp: "2026-10-12T00:00:00+00:00".into(),
        };
        let json = serde_json::to_value(&evt).unwrap();
        assert_eq!(json.get("name").and_then(|v| v.as_str()), Some("nginx"));
        assert_eq!(json.get("action").and_then(|v| v.as_str()), Some("start"));
        assert_eq!(json.get("state").and_then(|v| v.as_str()), Some("running"));
        assert_eq!(
            json.get("timestamp").and_then(|v| v.as_str()),
            Some("2026-10-12T00:00:00+00:00")
        );
        assert_eq!(
            json.as_object().map(|o| o.len()),
            Some(4),
            "ContainerEvent must serialize exactly the four contract fields"
        );
    }

    #[test]
    fn test_container_event_is_clone_and_debug() {
        let evt = ContainerEvent {
            name: "redis".into(),
            action: "die".into(),
            state: "exited".into(),
            timestamp: "t".into(),
        };
        let cloned = evt.clone();
        assert_eq!(cloned.name, "redis");
        assert_eq!(cloned.action, "die");
        assert!(format!("{:?}", evt).contains("redis"));
    }

    // ── Settings.telegram_topic_id (change: telegram-topic-id) ───

    /// `telegram_topic_id` is optional and `#[serde(default)]`: an empty JSON
    /// object must deserialize it to `None` (backwards-compatible with the
    /// settings already stored on disk).
    #[test]
    fn test_settings_topic_id_defaults_to_none_when_absent() {
        let s: Settings = serde_json::from_str("{}").expect("empty settings must deserialize");
        assert_eq!(s.telegram_topic_id, None);
    }

    /// Round-trip of the topic through serde (same shape as the other
    /// `telegram_*` string fields).
    #[test]
    fn test_settings_topic_id_roundtrip() {
        let s: Settings =
            serde_json::from_str(r#"{"telegram_topic_id":"42"}"#).expect("topic must deserialize");
        assert_eq!(s.telegram_topic_id.as_deref(), Some("42"));

        let value = serde_json::to_value(&s).expect("Settings must serialize");
        assert_eq!(value["telegram_topic_id"], "42");
    }

    /// An explicit `null` must also map to `None` (nothing was configured).
    #[test]
    fn test_settings_topic_id_null_is_none() {
        let s: Settings =
            serde_json::from_str(r#"{"telegram_topic_id":null}"#).expect("null topic is valid");
        assert_eq!(s.telegram_topic_id, None);
    }

    // ── PublicConfig.telegram_topic_id (change: telegram-topic-id) ───

    /// The topic is not a secret: `PublicConfig` exposes its value directly
    /// (no `*_set` indicator) so the UI can preload the field.
    #[test]
    fn test_public_config_exposes_topic_id() {
        let cfg = PublicConfig {
            oidc_configured: true,
            port: 3066,
            timezone: "UTC".into(),
            telegram_configured: true,
            matrix_configured: false,
            webhook_configured: false,
            valet_configured: false,
            telegram_token_set: true,
            matrix_token_set: false,
            valet_token_set: false,
            telegram_chat_id: Some("chat".into()),
            telegram_topic_id: Some("42".into()),
            matrix_homeserver: None,
            matrix_room: None,
            valet_url: None,
            version: "0.0.0".into(),
            build_date: "now".into(),
            repo_url: "https://example.test".into(),
        };
        let value = serde_json::to_value(&cfg).expect("PublicConfig must serialize");
        assert_eq!(value["telegram_topic_id"], "42");
    }

    // ── update-rollback-guard: modelos ───────────────────────

    /// Entradas de historial antiguas (sin `attempts`) deserializan con 0.
    #[test]
    fn test_update_history_entry_attempts_defaults_to_zero() {
        let entry: UpdateHistoryEntry = serde_json::from_str(
            r#"{"container":"c","image":"i","old_digest":"o","new_digest":"n","timestamp":"t","status":"updated","duration_ms":1}"#,
        )
        .expect("entry without attempts must deserialize");
        assert_eq!(entry.attempts, 0);
    }

    /// Un `attempts` explícito se conserva.
    #[test]
    fn test_update_history_entry_keeps_attempts() {
        let entry: UpdateHistoryEntry = serde_json::from_str(
            r#"{"container":"c","image":"i","old_digest":"o","new_digest":"n","timestamp":"t","status":"rolled-back","duration_ms":1,"attempts":3}"#,
        )
        .expect("entry with attempts must deserialize");
        assert_eq!(entry.attempts, 3);
    }

    /// Settings antiguos (sin los campos de la guarda) usan los defaults globales.
    #[test]
    fn test_settings_guard_defaults_when_fields_missing() {
        let s: Settings = serde_json::from_str("{}").expect("empty settings is valid");
        assert_eq!(s.update_max_start_attempts, 3);
        assert_eq!(s.update_start_attempt_interval_secs, 10);
        assert_eq!(s.update_health_window_secs, 30);
    }

    /// `Settings::default()` coincide con los defaults de serde.
    #[test]
    fn test_settings_default_matches_guard_defaults() {
        let s = Settings::default();
        assert_eq!(
            s.update_max_start_attempts,
            DEFAULT_UPDATE_MAX_START_ATTEMPTS
        );
        assert_eq!(
            s.update_start_attempt_interval_secs,
            DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS
        );
        assert_eq!(
            s.update_health_window_secs,
            DEFAULT_UPDATE_HEALTH_WINDOW_SECS
        );
    }

    /// Los overrides de `UpdatePolicyReq` son opcionales y se mapean a `Option`.
    #[test]
    fn test_update_policy_req_guard_overrides() {
        let without: UpdatePolicyReq = serde_json::from_str(
            r#"{"action":"pull-restart","cleanup_old_image":true,"rollback_on_failure":true}"#,
        )
        .expect("req without overrides must deserialize");
        assert_eq!(without.max_start_attempts, None);
        assert_eq!(without.health_window_secs, None);

        let with: UpdatePolicyReq = serde_json::from_str(
            r#"{"action":"pull-restart","cleanup_old_image":true,"rollback_on_failure":true,"max_start_attempts":5,"health_window_secs":60}"#,
        )
        .expect("req with overrides must deserialize");
        assert_eq!(with.max_start_attempts, Some(5));
        assert_eq!(with.health_window_secs, Some(60));
    }

    /// Los 3 campos de la guarda hacen roundtrip en `UpdateCheckConfig`.
    #[test]
    fn test_update_check_config_guard_roundtrip() {
        let config = UpdateCheckConfig {
            cron: "0 */6 * * *".into(),
            enabled: true,
            notify: false,
            last_run_at: None,
            next_run_at: None,
            pull_timeout_secs: None,
            check_interval_ms: None,
            update_max_start_attempts: 4,
            update_start_attempt_interval_secs: 12,
            update_health_window_secs: 45,
        };
        let json = serde_json::to_string(&config).expect("config must serialize");
        let back: UpdateCheckConfig = serde_json::from_str(&json).expect("config must deserialize");
        assert_eq!(back.update_max_start_attempts, 4);
        assert_eq!(back.update_start_attempt_interval_secs, 12);
        assert_eq!(back.update_health_window_secs, 45);
    }

    /// Sin `update_check_cron` el cron efectivo es el default canónico `0 6 * * *`.
    #[test]
    fn test_settings_without_cron_uses_canonical_default() {
        let s = Settings::default();
        assert_eq!(s.update_check_cron, None);
        assert_eq!(s.effective_update_check_cron(), DEFAULT_UPDATE_CHECK_CRON);
        assert_eq!(s.effective_update_check_cron(), "0 6 * * *");
    }

    /// Un cron vacío se normaliza al default canónico (misma semántica que `db.rs`).
    #[test]
    fn test_settings_empty_cron_uses_canonical_default() {
        let s = Settings {
            update_check_cron: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(s.update_check_cron.as_deref(), Some(""));
        assert_eq!(s.effective_update_check_cron(), DEFAULT_UPDATE_CHECK_CRON);
    }

    /// Sin `pull_timeout_secs` el timeout efectivo es el default canónico `600`.
    #[test]
    fn test_settings_without_pull_timeout_uses_canonical_default() {
        let s = Settings::default();
        assert_eq!(s.pull_timeout_secs, None);
        assert_eq!(s.effective_pull_timeout_secs(), DEFAULT_PULL_TIMEOUT_SECS);
        assert_eq!(s.effective_pull_timeout_secs(), 600);
    }

    /// Sin `check_interval_ms` el intervalo efectivo es el default canónico `2000`.
    #[test]
    fn test_settings_without_check_interval_uses_canonical_default() {
        let s = Settings::default();
        assert_eq!(s.check_interval_ms, None);
        assert_eq!(s.effective_check_interval_ms(), DEFAULT_CHECK_INTERVAL_MS);
        assert_eq!(s.effective_check_interval_ms(), 2000);
    }

    /// Valores explícitos de los settings prevalecen sobre los defaults canónicos.
    #[test]
    fn test_settings_explicit_update_values_win_over_defaults() {
        let s = Settings {
            update_check_cron: Some("0 3 * * *".into()),
            pull_timeout_secs: Some(120),
            check_interval_ms: Some(500),
            ..Default::default()
        };
        assert_eq!(s.effective_update_check_cron(), "0 3 * * *");
        assert_eq!(s.effective_pull_timeout_secs(), 120);
        assert_eq!(s.effective_check_interval_ms(), 500);
    }

    // ── default guard overrides (apply-default-guard-overrides) ──

    /// Compat: un JSON de `Settings` sin los campos nuevos deserializa a `None`
    /// (settings almacenados antes del cambio siguen cargando sin migración).
    #[test]
    fn test_settings_without_default_guard_overrides_deserializes_to_none() {
        let json = r#"{
            "update_max_start_attempts": 3,
            "update_start_attempt_interval_secs": 10,
            "update_health_window_secs": 30
        }"#;
        let s: Settings = serde_json::from_str(json)
            .expect("settings previos deben deserializar sin los campos nuevos");
        assert_eq!(s.default_max_start_attempts, None);
        assert_eq!(s.default_health_window_secs, None);
        assert_eq!(s.update_max_start_attempts, 3);
        assert_eq!(s.update_health_window_secs, 30);
    }

    /// El helper devuelve el par de overrides de la política por defecto.
    #[test]
    fn test_default_guard_overrides_returns_pair() {
        let s = Settings {
            default_max_start_attempts: Some(5),
            default_health_window_secs: Some(60),
            ..Default::default()
        };
        assert_eq!(s.default_guard_overrides(), (Some(5), Some(60)));
        assert_eq!(Settings::default().default_guard_overrides(), (None, None));
    }

    // ── fallback_update_policy (apply-default-guard-overrides) ──

    /// El fallback de un contenedor sin política toma acción/limpieza/rollback
    /// de la política por defecto y siembra los overrides de la guarda.
    #[test]
    fn test_fallback_update_policy_seeds_defaults_and_guard_overrides() {
        let s = Settings {
            default_update_action: Some("pull-restart-stack".into()),
            default_cleanup_old_image: Some(true),
            default_rollback_on_failure: Some(true),
            default_max_start_attempts: Some(5),
            default_health_window_secs: Some(60),
            ..Default::default()
        };
        let p = s.fallback_update_policy("web");
        assert_eq!(p.container, "web");
        assert_eq!(p.action, UpdateAction::PullRestartStack);
        assert!(p.cleanup_old_image);
        assert!(p.rollback_on_failure);
        assert!(!p.notify_events);
        assert_eq!(p.max_start_attempts, Some(5));
        assert_eq!(p.health_window_secs, Some(60));
    }

    /// Sin política por defecto ni overrides: `pull-restart`, limpieza/rollback
    /// a `false` y guarda sin override (`None`).
    #[test]
    fn test_fallback_update_policy_falls_back_to_sane_defaults() {
        let p = Settings::default().fallback_update_policy("web");
        assert_eq!(p.action, UpdateAction::PullRestart);
        assert!(!p.cleanup_old_image);
        assert!(!p.rollback_on_failure);
        assert!(!p.notify_events);
        assert_eq!(p.max_start_attempts, None);
        assert_eq!(p.health_window_secs, None);
    }

    /// Una acción por defecto inválida cae a `pull-restart` (no revienta).
    #[test]
    fn test_fallback_update_policy_invalid_action_falls_back_to_pull_restart() {
        let s = Settings {
            default_update_action: Some("no-such-action".into()),
            ..Default::default()
        };
        assert_eq!(
            s.fallback_update_policy("web").action,
            UpdateAction::PullRestart
        );
    }

    // ── paginación/filtrado del listado (container-list-pagination) ────

    /// Contenedor de test con proyecto de compose explícito.
    fn make_project_container(
        name: &str,
        state: &str,
        has_update: bool,
        project: Option<&str>,
    ) -> ContainerInfo {
        ContainerInfo {
            compose_project: project.map(str::to_string),
            ..make_container(name, state, has_update)
        }
    }

    /// Flota de `n` contenedores `running` sin actualización en el proyecto `alpha`.
    fn make_fleet(n: usize) -> Vec<ContainerInfo> {
        (0..n)
            .map(|i| make_project_container(&format!("c{i:02}"), "running", false, Some("alpha")))
            .collect()
    }

    fn normalize(query: ContainerListQuery) -> ContainerListParams {
        query.normalize().expect("query válida")
    }

    #[test]
    fn test_paginate_default_page_returns_first_twenty_of_twentyfive() {
        let fleet = make_fleet(25);
        let paged = paginate_containers(&fleet, &ContainerListParams::default());
        assert_eq!(paged.page, 1);
        assert_eq!(paged.page_size, 20);
        assert_eq!(paged.total, 25);
        assert_eq!(paged.items.len(), 20);
    }

    #[test]
    fn test_paginate_second_page_returns_remaining_five() {
        let fleet = make_fleet(25);
        let params = ContainerListParams {
            page: 2,
            ..Default::default()
        };
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.items.len(), 5);
        assert_eq!(paged.total, 25);
    }

    #[test]
    fn test_paginate_page_beyond_end_is_empty_and_keeps_real_total() {
        let fleet = make_fleet(25);
        let params = ContainerListParams {
            page: 99,
            ..Default::default()
        };
        let paged = paginate_containers(&fleet, &params);
        assert!(paged.items.is_empty());
        assert_eq!(paged.total, 25);
    }

    /// El offset debe saturar en vez de desbordar `usize` (relevante en 32 bits).
    #[test]
    fn test_page_offset_saturates_for_u32_max_page() {
        let offset = page_offset(u32::MAX, 50);
        assert!(
            offset >= u32::MAX as usize,
            "el offset debe quedar más allá de cualquier flota real, got {offset}"
        );
    }

    #[test]
    fn test_page_offset_matches_plain_arithmetic_when_no_overflow() {
        assert_eq!(page_offset(1, 20), 0);
        assert_eq!(page_offset(2, 20), 20);
        assert_eq!(page_offset(3, 10), 20);
    }

    /// `page = u32::MAX` no desborda ni panickea: página vacía con `total` real.
    #[test]
    fn test_paginate_page_u32_max_returns_empty_page_with_real_total() {
        let fleet = make_fleet(25);
        let params = ContainerListParams {
            page: u32::MAX,
            ..Default::default()
        };
        let paged = paginate_containers(&fleet, &params);
        assert!(paged.items.is_empty());
        assert_eq!(paged.total, 25);
        assert_eq!(paged.page, u32::MAX);
    }

    #[test]
    fn test_paginate_q_matches_name_case_insensitively() {
        let fleet = vec![
            make_container("web", "running", false),
            make_container("db", "running", false),
            make_container("cache", "running", false),
        ];
        let params = normalize(ContainerListQuery {
            q: Some("WEB".into()),
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 1);
        assert_eq!(paged.items[0].name, "web");
    }

    #[test]
    fn test_paginate_q_matches_image() {
        let fleet = vec![
            make_container("web", "running", false),
            make_container("db", "running", false),
        ];
        let params = normalize(ContainerListQuery {
            q: Some("web:lat".into()),
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 1);
        assert_eq!(paged.items[0].name, "web");
    }

    #[test]
    fn test_paginate_q_matches_compose_project() {
        let fleet = vec![
            make_project_container("web", "running", false, Some("shop")),
            make_project_container("db", "running", false, Some("blog")),
        ];
        let params = normalize(ContainerListQuery {
            q: Some("SHOP".into()),
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 1);
        assert_eq!(paged.items[0].name, "web");
    }

    #[test]
    fn test_paginate_filters_by_single_state_in_set() {
        let fleet = vec![
            make_container("web", "running", false),
            make_container("db", "exited", false),
            make_container("cache", "running", false),
        ];
        let params = normalize(ContainerListQuery {
            state: vec!["running".into()],
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 2);
        assert!(paged.items.iter().all(|c| c.state == "running"));
    }

    /// `state` es multi-valor: se conservan los contenedores cuyo estado está en
    /// el conjunto (semántica OR), descartando el resto.
    #[test]
    fn test_paginate_filters_by_state_set_or_semantics() {
        let fleet = vec![
            make_container("web", "running", false),
            make_container("db", "exited", false),
            make_container("cache", "paused", false),
            make_container("api", "running", false),
        ];
        let params = normalize(ContainerListQuery {
            state: vec!["running".into(), "exited".into()],
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 3);
        assert!(
            paged
                .items
                .iter()
                .all(|c| c.state == "running" || c.state == "exited"),
            "no debe incluir 'paused': {:?}",
            paged.items.iter().map(|c| &c.state).collect::<Vec<_>>()
        );
        assert!(!paged.items.iter().any(|c| c.name == "cache"));
    }

    /// Conjunto de estados vacío → no filtra por estado.
    #[test]
    fn test_paginate_empty_state_set_does_not_filter() {
        let fleet = vec![
            make_container("web", "running", false),
            make_container("db", "exited", false),
            make_container("cache", "paused", false),
        ];
        let params = normalize(ContainerListQuery {
            state: vec![],
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 3);
    }

    #[test]
    fn test_paginate_filters_by_has_update() {
        let fleet = vec![
            make_container("web", "running", true),
            make_container("db", "running", false),
            make_container("cache", "exited", true),
        ];
        let params = normalize(ContainerListQuery {
            has_update: Some(true),
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 2);
        assert!(paged.items.iter().all(|c| c.has_update));
    }

    #[test]
    fn test_paginate_combined_filters_and_pagination() {
        // 30 running, 12 con actualización → page_size=10&page=2 → 2 items.
        let mut fleet: Vec<ContainerInfo> = (0..30)
            .map(|i| make_project_container(&format!("c{i:02}"), "running", false, Some("alpha")))
            .collect();
        for c in fleet.iter_mut().take(12) {
            c.has_update = true;
        }
        fleet.push(make_container("other", "exited", true)); // no debe contar
        let params = normalize(ContainerListQuery {
            state: vec!["running".into()],
            has_update: Some(true),
            page: Some(2),
            page_size: Some(10),
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 12);
        assert_eq!(paged.items.len(), 2);
    }

    #[test]
    fn test_paginate_orders_by_project_then_name() {
        let fleet = vec![
            make_project_container("b", "running", false, Some("beta")),
            make_project_container("z", "running", false, Some("alpha")),
            make_project_container("a", "running", false, Some("alpha")),
            make_project_container("a", "running", false, Some("beta")),
        ];
        let paged = paginate_containers(&fleet, &ContainerListParams::default());
        let order: Vec<(Option<&str>, &str)> = paged
            .items
            .iter()
            .map(|c| (c.compose_project.as_deref(), c.name.as_str()))
            .collect();
        assert_eq!(
            order,
            vec![
                (Some("alpha"), "a"),
                (Some("alpha"), "z"),
                (Some("beta"), "a"),
                (Some("beta"), "b"),
            ]
        );
    }

    #[test]
    fn test_paginate_order_is_stable_across_calls() {
        let fleet = vec![
            make_project_container("web", "running", false, Some("beta")),
            make_project_container("api", "running", false, Some("alpha")),
            make_project_container("db", "running", false, Some("alpha")),
        ];
        let params = ContainerListParams {
            page_size: 10,
            ..Default::default()
        };
        let first = paginate_containers(&fleet, &params);
        let second = paginate_containers(&fleet, &params);
        let names =
            |p: &PagedContainers| p.items.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&first), names(&second));
    }

    #[test]
    fn test_paginate_projects_are_contiguous() {
        let fleet = vec![
            make_project_container("b1", "running", false, Some("beta")),
            make_project_container("a1", "running", false, Some("alpha")),
            make_project_container("b2", "running", false, Some("beta")),
            make_project_container("a2", "running", false, Some("alpha")),
        ];
        let paged = paginate_containers(&fleet, &ContainerListParams::default());
        let projects: Vec<&str> = paged
            .items
            .iter()
            .filter_map(|c| c.compose_project.as_deref())
            .collect();
        assert_eq!(projects, vec!["alpha", "alpha", "beta", "beta"]);
    }

    #[test]
    fn test_paginate_total_counts_only_filtered() {
        let fleet = vec![
            make_container("web", "running", false),
            make_container("db", "exited", false),
        ];
        let params = normalize(ContainerListQuery {
            state: vec!["exited".into()],
            ..Default::default()
        });
        let paged = paginate_containers(&fleet, &params);
        assert_eq!(paged.total, 1);
    }

    #[test]
    fn test_paginate_empty_input() {
        let paged = paginate_containers(&[], &ContainerListParams::default());
        assert!(paged.items.is_empty());
        assert_eq!(paged.total, 0);
        assert_eq!(paged.page, 1);
        assert_eq!(paged.page_size, 20);
    }

    // ── query: deserialización, normalización y validación ─────

    #[test]
    fn test_query_normalize_defaults() {
        let params = normalize(ContainerListQuery::default());
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
        assert_eq!(params.q, None);
        assert!(params.state.is_empty());
        assert_eq!(params.has_update, None);
    }

    #[test]
    fn test_query_deserializes_defaults_from_empty_query_string() {
        let uri: axum::http::Uri = "/api/containers".parse().expect("uri válida");
        let Query(query) = Query::<ContainerListQuery>::try_from_uri(&uri).expect("query vacía");
        let params = normalize(query);
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[test]
    fn test_query_deserializes_all_params() {
        let uri: axum::http::Uri =
            "/api/containers?page=2&page_size=50&q=web&state=running&has_update=true"
                .parse()
                .expect("uri válida");
        let Query(query) = Query::<ContainerListQuery>::try_from_uri(&uri).expect("query válida");
        let params = normalize(query);
        assert_eq!(params.page, 2);
        assert_eq!(params.page_size, 50);
        assert_eq!(params.q.as_deref(), Some("web"));
        assert_eq!(params.state, vec!["running".to_string()]);
        assert_eq!(params.has_update, Some(true));
    }

    /// `state` puede repetirse: `?state=running&state=exited` llega como dos valores.
    #[test]
    fn test_query_deserializes_repeated_state_values() {
        let uri: axum::http::Uri = "/api/containers?state=running&state=exited"
            .parse()
            .expect("uri válida");
        let Query(query) = Query::<ContainerListQuery>::try_from_uri(&uri).expect("query válida");
        let params = normalize(query);
        assert_eq!(
            params.state,
            vec!["running".to_string(), "exited".to_string()]
        );
    }

    /// Un único `state` sigue funcionando.
    #[test]
    fn test_query_deserializes_single_state_value() {
        let uri: axum::http::Uri = "/api/containers?state=running".parse().expect("uri válida");
        let Query(query) = Query::<ContainerListQuery>::try_from_uri(&uri).expect("query válida");
        let params = normalize(query);
        assert_eq!(params.state, vec!["running".to_string()]);
    }

    /// Sin `state` → conjunto vacío (no filtra).
    #[test]
    fn test_query_deserializes_absent_state_as_empty_set() {
        let uri: axum::http::Uri = "/api/containers?page=1".parse().expect("uri válida");
        let Query(query) = Query::<ContainerListQuery>::try_from_uri(&uri).expect("query válida");
        let params = normalize(query);
        assert!(params.state.is_empty());
    }

    /// `normalize` recorta cada valor, descarta los vacíos y deduplica.
    #[test]
    fn test_query_normalize_trims_and_dedups_state_values() {
        let query = ContainerListQuery {
            state: vec![
                "  ".into(),
                " running ".into(),
                "running".into(),
                "exited".into(),
                "".into(),
            ],
            ..Default::default()
        };
        let params = normalize(query);
        assert_eq!(
            params.state,
            vec!["running".to_string(), "exited".to_string()]
        );
    }

    #[test]
    fn test_query_normalize_page_zero_is_bad_request() {
        let query = ContainerListQuery {
            page: Some(0),
            ..Default::default()
        };
        let err = query.normalize().expect_err("page=0 debe rechazarse");
        assert!(matches!(&err, AppError::BadRequest(_)));
        assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_query_normalize_invalid_page_size_is_bad_request() {
        let query = ContainerListQuery {
            page_size: Some(30),
            ..Default::default()
        };
        let err = query.normalize().expect_err("page_size=30 debe rechazarse");
        assert!(matches!(&err, AppError::BadRequest(_)));
        assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_query_normalize_accepts_all_allowed_page_sizes() {
        for size in ALLOWED_PAGE_SIZES {
            let query = ContainerListQuery {
                page_size: Some(size),
                ..Default::default()
            };
            assert_eq!(normalize(query).page_size, size);
        }
    }

    #[test]
    fn test_query_normalize_trims_blank_filters() {
        let query = ContainerListQuery {
            q: Some("  ".into()),
            state: vec!["  ".into()],
            ..Default::default()
        };
        let params = normalize(query);
        assert_eq!(params.q, None);
        assert!(params.state.is_empty());
    }

    #[test]
    fn test_state_response_serialization_exposes_pagination_fields() {
        let response = StateResponse {
            containers: vec![],
            summary: compute_summary(&[]),
            progress: BatchProgress::default(),
            page: 3,
            page_size: 50,
            total: 123,
        };
        let json = serde_json::to_value(&response).expect("serializa");
        assert_eq!(json["page"], 3);
        assert_eq!(json["page_size"], 50);
        assert_eq!(json["total"], 123);
    }

    // ── histórico: query, normalización y envelope ─────────────
    // (change: update-history-pagination — tareas 1.1, 1.2 y 1.9)

    fn history_normalize(query: UpdateHistoryQuery) -> UpdateHistoryParams {
        query.normalize().expect("query válida")
    }

    /// 1.1 — defaults cuando la query string está vacía.
    #[test]
    fn test_history_query_deserializes_defaults_from_empty_query_string() {
        let uri: axum::http::Uri = "/api/history".parse().expect("uri válida");
        let Query(query) = Query::<UpdateHistoryQuery>::try_from_uri(&uri).expect("query vacía");
        let params = history_normalize(query);
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
        assert_eq!(params.q, None);
        assert!(params.status.is_empty());
    }

    /// 1.1 — escalares repetidos: *last-wins* (`?page=1&page=2` aplica 2).
    #[test]
    fn test_history_query_deserializes_repeated_scalar_last_wins() {
        let uri: axum::http::Uri = "/api/history?page=1&page=2".parse().expect("uri válida");
        let Query(query) = Query::<UpdateHistoryQuery>::try_from_uri(&uri).expect("query válida");
        let params = history_normalize(query);
        assert_eq!(params.page, 2);
    }

    /// 1.1 — parámetros desconocidos se ignoran (`pageSize` cae al default).
    #[test]
    fn test_history_query_deserializes_unknown_params_ignored() {
        let uri: axum::http::Uri = "/api/history?pageSize=50".parse().expect("uri válida");
        let Query(query) = Query::<UpdateHistoryQuery>::try_from_uri(&uri).expect("query válida");
        let params = history_normalize(query);
        assert_eq!(params.page_size, 20);
    }

    /// 1.1 — `status` es multi-valor: se acumulan todas sus ocurrencias.
    #[test]
    fn test_history_query_deserializes_repeated_status_values() {
        let uri: axum::http::Uri = "/api/history?status=error&status=rolled-back"
            .parse()
            .expect("uri válida");
        let Query(query) = Query::<UpdateHistoryQuery>::try_from_uri(&uri).expect("query válida");
        let params = history_normalize(query);
        assert_eq!(
            params.status,
            vec!["error".to_string(), "rolled-back".to_string()]
        );
    }

    /// 1.1 — todos los parámetros juntos.
    #[test]
    fn test_history_query_deserializes_all_params() {
        let uri: axum::http::Uri = "/api/history?page=3&page_size=50&q=web&status=success"
            .parse()
            .expect("uri válida");
        let Query(query) = Query::<UpdateHistoryQuery>::try_from_uri(&uri).expect("query válida");
        let params = history_normalize(query);
        assert_eq!(params.page, 3);
        assert_eq!(params.page_size, 50);
        assert_eq!(params.q.as_deref(), Some("web"));
        assert_eq!(params.status, vec!["success".to_string()]);
    }

    /// 1.2 — `page=0` → `400`.
    #[test]
    fn test_history_query_normalize_page_zero_is_bad_request() {
        let query = UpdateHistoryQuery {
            page: Some(0),
            ..Default::default()
        };
        let err = query.normalize().expect_err("page=0 debe rechazarse");
        assert!(matches!(&err, AppError::BadRequest(_)));
        assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    }

    /// 1.2 — `page_size=30` (fuera de {10,20,50}) → `400`.
    #[test]
    fn test_history_query_normalize_invalid_page_size_is_bad_request() {
        let query = UpdateHistoryQuery {
            page_size: Some(30),
            ..Default::default()
        };
        let err = query.normalize().expect_err("page_size=30 debe rechazarse");
        assert!(matches!(&err, AppError::BadRequest(_)));
        assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    }

    /// 1.2 — `page=abc` es error de deserialización (axum responde `400`).
    #[test]
    fn test_history_query_page_abc_is_deserialization_error() {
        let uri: axum::http::Uri = "/api/history?page=abc".parse().expect("uri válida");
        assert!(Query::<UpdateHistoryQuery>::try_from_uri(&uri).is_err());
    }

    /// 1.2 — `page_size=abc` es error de deserialización (axum responde `400`).
    #[test]
    fn test_history_query_page_size_abc_is_deserialization_error() {
        let uri: axum::http::Uri = "/api/history?page_size=abc".parse().expect("uri válida");
        assert!(Query::<UpdateHistoryQuery>::try_from_uri(&uri).is_err());
    }

    /// 1.2 — los tres tamaños permitidos pasan la validación.
    #[test]
    fn test_history_query_normalize_accepts_all_allowed_page_sizes() {
        for size in ALLOWED_PAGE_SIZES {
            let query = UpdateHistoryQuery {
                page_size: Some(size),
                ..Default::default()
            };
            assert_eq!(history_normalize(query).page_size, size);
        }
    }

    /// 1.9 — el envelope serializa exactamente `{items,total,page,page_size}`.
    #[test]
    fn test_paged_update_history_serializes_envelope() {
        let page = PagedUpdateHistory {
            items: vec![UpdateHistoryEntry {
                container: "web".into(),
                image: "nginx".into(),
                old_digest: "sha256:old".into(),
                new_digest: "sha256:new".into(),
                timestamp: "2026-01-01T00:00:00".into(),
                status: "success".into(),
                duration_ms: 10,
                attempts: 0,
            }],
            total: 42,
            page: 2,
            page_size: 20,
        };
        let json = serde_json::to_value(&page).expect("serializa");
        let obj = json.as_object().expect("objeto JSON");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["items", "page", "page_size", "total"]);
        assert_eq!(json["total"], 42);
        assert_eq!(json["page"], 2);
        assert_eq!(json["page_size"], 20);
        assert_eq!(json["items"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["items"][0]["container"], "web");
    }
}
