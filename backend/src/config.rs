use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::models::{AlertConfig, ScheduleTask};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub scan_interval_secs: Option<u64>,
    #[serde(default)]
    pub allowed_containers: Option<Vec<String>>,
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
    pub oidc_issuer_url: Option<String>,
    #[serde(default)]
    pub oidc_client_id: Option<String>,
    #[serde(default)]
    pub oidc_client_secret: Option<String>,
    #[serde(default)]
    pub oidc_redirect_url: Option<String>,
    #[serde(default)]
    pub alerts: Option<Vec<AlertConfig>>,
    #[serde(default)]
    pub schedule: Option<Vec<ScheduleTask>>,
}

/// Intenta leer un secreto de Podman montado en `/run/secrets/<name>`.
/// Si el archivo no existe o no se puede leer, devuelve `None`.
fn read_secret(name: &str) -> Option<String> {
    let path = Path::new("/run/secrets").join(name);
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl Config {
    pub fn load() -> Self {
        let mut cfg: Config = fs::read_to_string("config.yaml")
            .ok()
            .and_then(|c| serde_yaml::from_str(&c).ok())
            .unwrap_or_default();
        if let Ok(v) = std::env::var("HOST") {
            cfg.host = Some(v);
        }
        if let Ok(v) = std::env::var("PORT") {
            cfg.port = v.parse().ok();
        }
        // OIDC: prioridad a secretos de Podman, fallback a env vars
        cfg.oidc_issuer_url = read_secret("alloy-oidc-issuer")
            .or_else(|| read_secret("oidc_issuer_url"))
            .or_else(|| std::env::var("OIDC_ISSUER_URL").ok());
        cfg.oidc_client_id = read_secret("alloy-oidc-client-id")
            .or_else(|| read_secret("oidc_client_id"))
            .or_else(|| std::env::var("OIDC_CLIENT_ID").ok());
        cfg.oidc_client_secret = read_secret("alloy-oidc-client-secret")
            .or_else(|| read_secret("oidc_client_secret"))
            .or_else(|| std::env::var("OIDC_CLIENT_SECRET").ok());
        if let Ok(v) = std::env::var("OIDC_REDIRECT_URL") {
            cfg.oidc_redirect_url = Some(v);
        }
        // Telegram: prioridad a secretos de Podman, fallback a env vars
        cfg.telegram_token =
            read_secret("telegram_token").or_else(|| std::env::var("TELEGRAM_TOKEN").ok());
        if let Ok(v) = std::env::var("TELEGRAM_CHAT_ID") {
            cfg.telegram_chat_id = Some(v);
        }
        // Matrix: prioridad a secretos de Podman, fallback a env vars
        cfg.matrix_token =
            read_secret("matrix_token").or_else(|| std::env::var("MATRIX_TOKEN").ok());
        if let Ok(v) = std::env::var("MATRIX_HOMESERVER") {
            cfg.matrix_homeserver = Some(v);
        }
        if let Ok(v) = std::env::var("MATRIX_ROOM") {
            cfg.matrix_room = Some(v);
        }
        cfg
    }

    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("0.0.0.0")
    }
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(3066)
    }
    pub fn scan_interval(&self) -> u64 {
        self.scan_interval_secs.unwrap_or(5)
    }
    pub fn auto_update(&self) -> bool {
        self.auto_update_enabled.unwrap_or(false)
    }
    pub fn auto_update_interval(&self) -> u64 {
        self.auto_update_interval_hours.unwrap_or(6)
    }

    pub fn oidc_issuer(&self) -> &str {
        self.oidc_issuer_url
            .as_deref()
            .expect("OIDC_ISSUER_URL is required")
    }
    pub fn oidc_client_id(&self) -> &str {
        self.oidc_client_id
            .as_deref()
            .expect("OIDC_CLIENT_ID is required")
    }
    pub fn oidc_client_secret(&self) -> &str {
        self.oidc_client_secret
            .as_deref()
            .expect("OIDC_CLIENT_SECRET is required")
    }
    pub fn oidc_redirect_url(&self) -> &str {
        self.oidc_redirect_url
            .as_deref()
            .expect("OIDC_REDIRECT_URL is required")
    }
}

// ── API handlers ─────────────────────────────────────────────
use axum::{extract::State, response::Json, routing::get, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::{PublicConfig, Settings, UpdateSettingsReq, FILE_SETTINGS};
use crate::persistence::json_writer;
use crate::state::AppState;

async fn config_handler(
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
) -> Json<PublicConfig> {
    let s = settings.lock().await;
    let tg_token = s.telegram_token.as_ref().or(config.telegram_token.as_ref());
    let tg_chat_id = s
        .telegram_chat_id
        .clone()
        .or_else(|| config.telegram_chat_id.clone());
    let mx_homeserver = s
        .matrix_homeserver
        .clone()
        .or_else(|| config.matrix_homeserver.clone());
    let mx_token = s.matrix_token.as_ref().or(config.matrix_token.as_ref());
    let mx_room = s.matrix_room.clone().or_else(|| config.matrix_room.clone());
    Json(PublicConfig {
        oidc_configured: true,
        port: config.port(),
        auto_update_enabled: s
            .auto_update_enabled
            .unwrap_or_else(|| config.auto_update()),
        auto_update_interval_hours: s
            .auto_update_interval_hours
            .unwrap_or_else(|| config.auto_update_interval()),
        telegram_configured: tg_token.is_some(),
        telegram_token_set: tg_token.is_some(),
        telegram_chat_id: tg_chat_id,
        matrix_configured: mx_homeserver.is_some(),
        matrix_token_set: mx_token.is_some(),
        matrix_homeserver: mx_homeserver,
        matrix_room: mx_room,
        allowed_containers: config.allowed_containers.clone(),
    })
}

async fn update_config_h(
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
    Json(body): Json<UpdateSettingsReq>,
) -> Json<PublicConfig> {
    {
        let mut s = settings.lock().await;
        if let Some(v) = body.auto_update_enabled {
            s.auto_update_enabled = Some(v);
        }
        if let Some(v) = body.auto_update_interval_hours {
            s.auto_update_interval_hours = Some(v);
        }
        if let Some(v) = body.telegram_token {
            if v.is_empty() {
                s.telegram_token = None;
            } else {
                s.telegram_token = Some(v);
            }
        }
        if let Some(v) = body.telegram_chat_id {
            if v.is_empty() {
                s.telegram_chat_id = None;
            } else {
                s.telegram_chat_id = Some(v);
            }
        }
        if let Some(v) = body.matrix_homeserver {
            if v.is_empty() {
                s.matrix_homeserver = None;
            } else {
                s.matrix_homeserver = Some(v);
            }
        }
        if let Some(v) = body.matrix_token {
            if v.is_empty() {
                s.matrix_token = None;
            } else {
                s.matrix_token = Some(v);
            }
        }
        if let Some(v) = body.matrix_room {
            if v.is_empty() {
                s.matrix_room = None;
            } else {
                s.matrix_room = Some(v);
            }
        }
        json_writer().save(FILE_SETTINGS, &*s).await;
    }
    config_handler(State(config), State(settings)).await
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/config", get(config_handler).put(update_config_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_host() {
        let cfg = Config::default();
        assert_eq!(cfg.host(), "0.0.0.0");
    }

    #[test]
    fn test_custom_host() {
        let cfg = Config {
            host: Some("127.0.0.1".into()),
            ..Default::default()
        };
        assert_eq!(cfg.host(), "127.0.0.1");
    }

    #[test]
    fn test_default_port() {
        let cfg = Config::default();
        assert_eq!(cfg.port(), 3066);
    }

    #[test]
    fn test_custom_port() {
        let cfg = Config {
            port: Some(8080),
            ..Default::default()
        };
        assert_eq!(cfg.port(), 8080);
    }

    #[test]
    fn test_default_scan_interval() {
        let cfg = Config::default();
        assert_eq!(cfg.scan_interval(), 5);
    }

    #[test]
    fn test_custom_scan_interval() {
        let cfg = Config {
            scan_interval_secs: Some(10),
            ..Default::default()
        };
        assert_eq!(cfg.scan_interval(), 10);
    }

    #[test]
    fn test_default_auto_update_disabled() {
        let cfg = Config::default();
        assert!(!cfg.auto_update());
    }

    #[test]
    fn test_auto_update_enabled() {
        let cfg = Config {
            auto_update_enabled: Some(true),
            ..Default::default()
        };
        assert!(cfg.auto_update());
    }

    #[test]
    fn test_default_auto_update_interval() {
        let cfg = Config::default();
        assert_eq!(cfg.auto_update_interval(), 6);
    }

    #[test]
    fn test_custom_auto_update_interval() {
        let cfg = Config {
            auto_update_interval_hours: Some(12),
            ..Default::default()
        };
        assert_eq!(cfg.auto_update_interval(), 12);
    }

    #[test]
    #[should_panic(expected = "OIDC_ISSUER_URL is required")]
    fn test_oidc_issuer_panics_when_empty() {
        let cfg = Config::default();
        cfg.oidc_issuer();
    }

    #[test]
    fn test_config_yaml_deserialize() {
        let yaml = r#"
host: "127.0.0.1"
port: 8080
scan_interval_secs: 10
auto_update_enabled: true
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.host(), "127.0.0.1");
        assert_eq!(cfg.port(), 8080);
        assert_eq!(cfg.scan_interval(), 10);
        assert!(cfg.auto_update());
    }
}
