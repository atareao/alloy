use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

#[derive(Clone, Debug, Serialize)]
pub struct StateEvent {
    pub containers: Vec<ContainerInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
}

#[derive(Clone, Debug, Serialize)]
pub struct NotifEvent {
    pub container: String,
    pub status: String,
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
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_token: Option<String>,
    pub matrix_room: Option<String>,
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
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub telegram_token: Option<String>,
    #[serde(default)]
    pub telegram_chat_id: Option<String>,
    #[serde(default)]
    pub matrix_homeserver: Option<String>,
    #[serde(default)]
    pub matrix_token: Option<String>,
    #[serde(default)]
    pub matrix_room: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,

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

    /// Timeout en segundos para el pull de imágenes. Default: 1800 (30 min).
    /// Aumentar para imágenes muy grandes (>500MB) o conexiones lentas.
    #[serde(default)]
    pub pull_timeout_secs: Option<u64>,

    /// Intervalo en milisegundos entre verificaciones de digest de cada contenedor
    /// en check_all_h y update_check_worker. Default: 2000 (2s).
    /// Aumentar para evitar rate limiting en registries públicos.
    #[serde(default)]
    pub check_interval_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePolicy {
    pub container: String,
    pub action: UpdateAction,
    pub cleanup_old_image: bool,
    pub rollback_on_failure: bool,
    #[serde(default = "default_notify_events")]
    pub notify_events: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePolicyReq {
    pub action: UpdateAction,
    pub cleanup_old_image: bool,
    pub rollback_on_failure: bool,
    #[serde(default = "default_notify_events")]
    pub notify_events: bool,
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
    /// Timeout en segundos para pull de imágenes. Default: 1800 (30 min).
    #[serde(default)]
    pub pull_timeout_secs: Option<u64>,
    /// Intervalo en ms entre verificaciones de cada contenedor.
    #[serde(default)]
    pub check_interval_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateSettingsReq {
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_token: Option<String>,
    pub matrix_room: Option<String>,
    pub webhook_url: Option<String>,
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
pub const LABEL_COMPOSE_CONFIG_FILES: &str = "com.docker.compose.project.config_files";
pub const LABEL_COMPOSE_WORKING_DIR: &str = "com.docker.compose.project.working_dir";

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
}

#[derive(Clone, Debug, Serialize)]
pub struct StateResponse {
    pub containers: Vec<ContainerInfo>,
    pub summary: ContainerSummary,
    #[serde(default)]
    pub progress: BatchProgress,
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
        };
        let response = StateResponse {
            containers,
            summary,
            progress,
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
    }
}
