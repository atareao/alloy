use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub oidc_issuer_url: Option<String>,
    #[serde(default)]
    pub oidc_client_id: Option<String>,
    #[serde(default)]
    pub oidc_client_secret: Option<String>,
    #[serde(default)]
    pub oidc_redirect_url: Option<String>,
    /// Idle timeout in minutes. Auto-logout after this period of inactivity.
    /// Default: 30 minutes.
    #[serde(default)]
    pub session_idle_timeout_minutes: Option<u64>,
    /// Max session duration in hours. Forces logout after this time since login.
    /// Default: 24 hours.
    #[serde(default)]
    pub session_max_duration_hours: Option<u64>,
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
        cfg
    }

    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("0.0.0.0")
    }
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(3066)
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
    pub fn session_idle_timeout_minutes(&self) -> u64 {
        self.session_idle_timeout_minutes.unwrap_or(30)
    }
    pub fn session_max_duration_hours(&self) -> u64 {
        self.session_max_duration_hours.unwrap_or(24)
    }
}

// ── API handlers ─────────────────────────────────────────────
use axum::{
    extract::State,
    response::Json,
    routing::{get, put},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::db::DbPool;
use crate::models::{PublicConfig, Settings, UpdateSettingsReq};
use crate::state::AppState;

async fn config_handler(State(settings): State<Arc<Mutex<Settings>>>) -> Json<PublicConfig> {
    let s = settings.lock().await;
    Json(PublicConfig {
        oidc_configured: true,
        port: 3066,
        timezone: crate::timezone::timezone_abbr(),
        // A channel is "configured" only when it can actually send: the
        // conjunction of the fields its notifier requires.
        telegram_configured: s.telegram_token.is_some() && s.telegram_chat_id.is_some(),
        matrix_configured: s.matrix_homeserver.is_some()
            && s.matrix_token.is_some()
            && s.matrix_room.is_some(),
        webhook_configured: s.webhook_url.is_some(),
        valet_configured: s.valet_url.is_some() && s.valet_token.is_some(),
        telegram_token_set: s.telegram_token.is_some(),
        matrix_token_set: s.matrix_token.is_some(),
        valet_token_set: s.valet_token.is_some(),
        telegram_chat_id: s.telegram_chat_id.clone(),
        matrix_homeserver: s.matrix_homeserver.clone(),
        matrix_room: s.matrix_room.clone(),
        valet_url: s.valet_url.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_date: env!("BUILD_DATE").to_string(),
        repo_url: "https://github.com/atareao/alloy".to_string(),
    })
}

async fn update_config_h(
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    Json(body): Json<UpdateSettingsReq>,
) -> Json<PublicConfig> {
    {
        let mut s = settings.lock().await;
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
        if let Some(v) = body.webhook_url {
            if v.is_empty() {
                s.webhook_url = None;
            } else {
                s.webhook_url = Some(v);
            }
        }
        if let Some(v) = body.valet_url {
            if v.is_empty() {
                s.valet_url = None;
            } else {
                s.valet_url = Some(v);
            }
        }
        if let Some(v) = body.valet_token {
            if v.is_empty() {
                s.valet_token = None;
            } else {
                s.valet_token = Some(v);
            }
        }
        let conn = db_pool.get().await.unwrap();
        let _ = db::save_settings(&conn.lock().unwrap(), &s);
    }
    config_handler(State(settings)).await
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/config", get(config_handler))
        .route("/api/config", put(update_config_h))
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
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.host(), "127.0.0.1");
        assert_eq!(cfg.port(), 8080);
    }

    // ── Redaction of notification secrets ────────────────────

    /// `GET /api/config` must not leak the plaintext notification tokens;
    /// it may only expose the boolean `*_configured` indicators.
    #[tokio::test]
    async fn test_config_handler_hides_secret_tokens() {
        let settings = Arc::new(Mutex::new(Settings {
            telegram_token: Some("tg-secret".into()),
            telegram_chat_id: Some("12345".into()),
            matrix_homeserver: Some("https://matrix.example".into()),
            matrix_token: Some("mx-secret".into()),
            matrix_room: Some("!room:example".into()),
            valet_url: Some("https://valet.example".into()),
            valet_token: Some("val-secret".into()),
            ..Default::default()
        }));

        let Json(cfg) = config_handler(State(settings)).await;
        let value = serde_json::to_value(&cfg).unwrap();

        assert!(
            value.get("telegram_token").is_none(),
            "telegram_token must not be serialized: {value}"
        );
        assert!(
            value.get("matrix_token").is_none(),
            "matrix_token must not be serialized: {value}"
        );
        assert!(
            value.get("valet_token").is_none(),
            "valet_token must not be serialized: {value}"
        );

        assert_eq!(value["telegram_configured"].as_bool(), Some(true));
        assert_eq!(value["matrix_configured"].as_bool(), Some(true));
        assert_eq!(value["valet_configured"].as_bool(), Some(true));

        // Non-secret fields stay exposed.
        assert_eq!(value["telegram_chat_id"], "12345");
        assert_eq!(value["matrix_homeserver"], "https://matrix.example");
        assert_eq!(value["valet_url"], "https://valet.example");
    }

    /// `PUT /api/config` with the token field absent (e.g. UI left it blank)
    /// must preserve the stored token.
    #[tokio::test]
    async fn test_update_config_preserves_token_when_absent() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings {
            telegram_token: Some("keep-me".into()),
            matrix_token: Some("keep-matrix".into()),
            valet_token: Some("keep-valet".into()),
            ..Default::default()
        }));

        let body = UpdateSettingsReq {
            telegram_token: None,
            telegram_chat_id: Some("chat".into()),
            matrix_homeserver: None,
            matrix_token: None,
            matrix_room: None,
            webhook_url: None,
            valet_url: None,
            valet_token: None,
        };

        let _ = update_config_h(State(settings.clone()), State(db_pool), Json(body)).await;

        let s = settings.lock().await;
        assert_eq!(s.telegram_token.as_deref(), Some("keep-me"));
        assert_eq!(s.matrix_token.as_deref(), Some("keep-matrix"));
        assert_eq!(s.valet_token.as_deref(), Some("keep-valet"));
    }

    // ── `*_configured` mirrors real send capability ──────────

    /// Serialize the handler output once for a given `Settings`.
    async fn public_config_value(settings: Settings) -> serde_json::Value {
        let Json(cfg) = config_handler(State(Arc::new(Mutex::new(settings)))).await;
        serde_json::to_value(&cfg).unwrap()
    }

    #[tokio::test]
    async fn test_telegram_configured_requires_token_and_chat_id() {
        // Token only ⇒ not configured, but token_set reflects the stored token.
        let value = public_config_value(Settings {
            telegram_token: Some("tg".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["telegram_configured"].as_bool(), Some(false));
        assert_eq!(value["telegram_token_set"].as_bool(), Some(true));

        // Token + chat_id ⇒ configured.
        let value = public_config_value(Settings {
            telegram_token: Some("tg".into()),
            telegram_chat_id: Some("123".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["telegram_configured"].as_bool(), Some(true));
        assert_eq!(value["telegram_token_set"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn test_matrix_configured_requires_homeserver_token_and_room() {
        // Homeserver only ⇒ not configured.
        let value = public_config_value(Settings {
            matrix_homeserver: Some("https://matrix.example".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["matrix_configured"].as_bool(), Some(false));
        assert_eq!(value["matrix_token_set"].as_bool(), Some(false));

        // Homeserver + token but no room ⇒ not configured, yet token_set=true.
        let value = public_config_value(Settings {
            matrix_homeserver: Some("https://matrix.example".into()),
            matrix_token: Some("mx".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["matrix_configured"].as_bool(), Some(false));
        assert_eq!(value["matrix_token_set"].as_bool(), Some(true));

        // Complete ⇒ configured.
        let value = public_config_value(Settings {
            matrix_homeserver: Some("https://matrix.example".into()),
            matrix_token: Some("mx".into()),
            matrix_room: Some("!room:example".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["matrix_configured"].as_bool(), Some(true));
        assert_eq!(value["matrix_token_set"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn test_valet_configured_requires_url_and_token() {
        // URL only ⇒ not configured.
        let value = public_config_value(Settings {
            valet_url: Some("https://valet.example".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["valet_configured"].as_bool(), Some(false));
        assert_eq!(value["valet_token_set"].as_bool(), Some(false));

        // Complete ⇒ configured.
        let value = public_config_value(Settings {
            valet_url: Some("https://valet.example".into()),
            valet_token: Some("val".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["valet_configured"].as_bool(), Some(true));
        assert_eq!(value["valet_token_set"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn test_webhook_configured_requires_url() {
        // URL absent ⇒ not configured.
        let value = public_config_value(Settings::default()).await;
        assert_eq!(value["webhook_configured"].as_bool(), Some(false));

        let value = public_config_value(Settings {
            webhook_url: Some("https://hook.example".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(value["webhook_configured"].as_bool(), Some(true));
    }

    /// `PUT /api/config` with the token set to `""` must delete the stored
    /// token (`None`).
    #[tokio::test]
    async fn test_update_config_clears_token_when_empty() {
        let db_pool = db::test_pool();
        let settings = Arc::new(Mutex::new(Settings {
            telegram_token: Some("old-tg".into()),
            matrix_token: Some("old-mx".into()),
            valet_token: Some("old-val".into()),
            ..Default::default()
        }));

        let body = UpdateSettingsReq {
            telegram_token: Some(String::new()),
            telegram_chat_id: None,
            matrix_homeserver: None,
            matrix_token: Some(String::new()),
            matrix_room: None,
            webhook_url: None,
            valet_url: None,
            valet_token: Some(String::new()),
        };

        let _ = update_config_h(State(settings.clone()), State(db_pool), Json(body)).await;

        let s = settings.lock().await;
        assert!(s.telegram_token.is_none());
        assert!(s.matrix_token.is_none());
        assert!(s.valet_token.is_none());
    }
}
