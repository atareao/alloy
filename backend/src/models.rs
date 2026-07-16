use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use uuid::Uuid;

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
    pub monitored: bool,
    pub compose_project: Option<String>,
    pub ports: Vec<String>,
    pub traefik_url: Option<String>,
    pub registry_url: String,
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
    pub auto_update_enabled: bool,
    pub auto_update_interval_hours: u64,
    pub telegram_configured: bool,
    pub matrix_configured: bool,
    pub webhook_configured: bool,
    pub allowed_containers: Option<Vec<String>>,
    pub telegram_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub matrix_homeserver: Option<String>,
    pub matrix_token: Option<String>,
    pub matrix_room: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,
    pub name: String,
    pub email: String,
    pub exp: usize,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertConfig {
    #[serde(default = "default_alert_id")]
    pub id: String,
    pub container: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub notify_via: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateAlert {
    pub container: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub notify_via: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateSchedule {
    pub container: String,
    #[serde(default = "default_target_type")]
    pub target_type: String,
    pub cron: String,
    pub action: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub notify: bool,
    #[serde(default = "default_cleanup")]
    pub cleanup: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub auto_update_enabled: Option<bool>,
    #[serde(default)]
    pub auto_update_interval_hours: Option<u64>,
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
    pub monitored_containers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateSettingsReq {
    pub auto_update_enabled: Option<bool>,
    pub auto_update_interval_hours: Option<u64>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleTask {
    #[serde(default = "default_schedule_id")]
    pub id: String,
    pub container: String,
    #[serde(default = "default_target_type")]
    pub target_type: String,
    pub cron: String,
    pub action: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub notify: bool,
    #[serde(default = "default_cleanup")]
    pub cleanup: String,
}

pub fn default_target_type() -> String {
    "container".to_string()
}

pub fn default_cleanup() -> String {
    "none".to_string()
}

pub fn default_alert_id() -> String {
    Uuid::new_v4().to_string()
}
pub fn default_enabled() -> bool {
    true
}
pub fn default_schedule_id() -> String {
    Uuid::new_v4().to_string()
}

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

// ── Constants ──────────────────────────────────────────────

pub const ALL_CONTAINERS: &str = "*";

pub const LABEL_COMPOSE_PROJECT: &str = "com.docker.compose.project";
pub const LABEL_COMPOSE_SERVICE: &str = "com.docker.compose.service";
pub const LABEL_COMPOSE_CONFIG_FILES: &str = "com.docker.compose.project.config_files";
pub const LABEL_COMPOSE_WORKING_DIR: &str = "com.docker.compose.project.working_dir";

pub fn strip_name(name: &str) -> String {
    name.trim_start_matches('/').to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

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
    fn test_default_alert_id_is_uuid() {
        let id = default_alert_id();
        assert!(!id.is_empty());
        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn test_default_enabled() {
        assert!(default_enabled());
    }

    #[test]
    fn test_default_schedule_id_is_uuid() {
        let id = default_schedule_id();
        assert!(!id.is_empty());
        assert!(Uuid::parse_str(&id).is_ok());
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

    #[test]
    fn test_alert_config_defaults() {
        let alert = AlertConfig {
            id: String::new(),
            container: "web".into(),
            enabled: false,
            notify_via: vec![],
        };
        assert_eq!(alert.container, "web");
    }
}
