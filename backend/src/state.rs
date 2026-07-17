use bollard::Docker;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::config::Config;
use crate::db::DbPool;
use crate::models::*;
use crate::workers::CachedContainers;

pub type OidcStates = Arc<Mutex<HashMap<String, (String, std::time::Instant)>>>;

pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create reqwest::Client")
    })
}

/// JWT Validator estilo PocketID (oxinbox).
/// Obtiene las JWKS del issuer y valida tokens Bearer con RSA256.
#[derive(Clone)]
pub struct JwtValidator {
    jwks: Arc<RwLock<Vec<jsonwebtoken::DecodingKey>>>,
    issuer: String,
    client_id: String,
}

impl JwtValidator {
    pub fn new(issuer: &str, client_id: &str) -> Self {
        Self {
            jwks: Arc::new(RwLock::new(Vec::new())),
            issuer: issuer.to_string(),
            client_id: client_id.to_string(),
        }
    }

    pub async fn fetch_jwks(&self) -> Result<(), String> {
        let jwks_url = format!(
            "{}/.well-known/jwks.json",
            self.issuer.trim_end_matches('/')
        );
        let client = http_client();
        let resp: serde_json::Value = client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| format!("failed to fetch JWKS: {e}"))?
            .json()
            .await
            .map_err(|e| format!("failed to parse JWKS response: {e}"))?;

        let keys = resp["keys"]
            .as_array()
            .ok_or_else(|| "JWKS response missing 'keys' array".to_string())?;

        let mut decoding_keys = Vec::new();
        for key in keys {
            if let (Some(n), Some(e)) = (
                key["n"].as_str().and_then(|s| {
                    base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, s)
                        .ok()
                }),
                key["e"].as_str().and_then(|s| {
                    base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, s)
                        .ok()
                }),
            ) {
                let dk = jsonwebtoken::DecodingKey::from_rsa_raw_components(&n, &e);
                decoding_keys.push(dk);
            }
        }

        tracing::info!(
            count = decoding_keys.len(),
            "JWKS fetched from {}",
            jwks_url
        );
        *self.jwks.write().await = decoding_keys;
        Ok(())
    }

    pub async fn validate_token(&self, token: &str) -> Result<JwtClaims, String> {
        let keys = {
            let jwks = self.jwks.read().await;
            if jwks.is_empty() {
                // Auto-fetch on first use
                drop(jwks);
                self.fetch_jwks().await?;
                return Box::pin(self.validate_token(token)).await;
            }
            jwks.clone()
        };

        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.client_id]);
        validation.validate_exp = true;

        for key in &keys {
            if let Ok(data) = jsonwebtoken::decode::<JwtClaims>(token, key, &validation) {
                return Ok(data.claims);
            }
        }
        Err("no matching JWK found for token".to_string())
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct OidcMetadata {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub issuer: String,
    #[serde(rename = "jwks_uri")]
    pub jwks_uri: String,
}

#[derive(Clone)]
pub struct AppState {
    pub docker: Docker,
    pub config: Config,
    pub tx: broadcast::Sender<StateEvent>,
    pub update_tx: broadcast::Sender<UpdateProgress>,
    pub notif_tx: broadcast::Sender<NotifEvent>,
    pub oidc_states: OidcStates,
    pub oidc_metadata: Option<OidcMetadata>,
    pub jwt_validator: JwtValidator,
    pub update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    pub schedules: Arc<Mutex<Vec<ScheduleTask>>>,

    pub cached_containers: CachedContainers,
    pub settings: Arc<Mutex<Settings>>,
    pub db: DbPool,
}

// FromRef implementations so handlers can extract individual types via State extractor
impl axum::extract::FromRef<AppState> for Docker {
    fn from_ref(state: &AppState) -> Self {
        state.docker.clone()
    }
}

impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl axum::extract::FromRef<AppState> for broadcast::Sender<StateEvent> {
    fn from_ref(state: &AppState) -> Self {
        state.tx.clone()
    }
}

impl axum::extract::FromRef<AppState> for broadcast::Sender<UpdateProgress> {
    fn from_ref(state: &AppState) -> Self {
        state.update_tx.clone()
    }
}

impl axum::extract::FromRef<AppState> for broadcast::Sender<NotifEvent> {
    fn from_ref(state: &AppState) -> Self {
        state.notif_tx.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<Mutex<Vec<UpdateHistoryEntry>>> {
    fn from_ref(state: &AppState) -> Self {
        state.update_history.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<Mutex<Vec<ScheduleTask>>> {
    fn from_ref(state: &AppState) -> Self {
        state.schedules.clone()
    }
}

impl axum::extract::FromRef<AppState> for OidcStates {
    fn from_ref(state: &AppState) -> Self {
        state.oidc_states.clone()
    }
}

impl axum::extract::FromRef<AppState> for Option<OidcMetadata> {
    fn from_ref(state: &AppState) -> Self {
        state.oidc_metadata.clone()
    }
}

impl axum::extract::FromRef<AppState> for JwtValidator {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_validator.clone()
    }
}

impl axum::extract::FromRef<AppState> for CachedContainers {
    fn from_ref(state: &AppState) -> Self {
        state.cached_containers.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<Mutex<Settings>> {
    fn from_ref(state: &AppState) -> Self {
        state.settings.clone()
    }
}

impl axum::extract::FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── OidcMetadata deserialization ─────────────────────────

    #[test]
    fn test_oidc_metadata_deserialize_full() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "issuer": "https://auth.example.com",
            "jwks_uri": "https://auth.example.com/.well-known/jwks.json"
        }"#;
        let meta: OidcMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(
            meta.authorization_endpoint,
            "https://auth.example.com/authorize"
        );
        assert_eq!(meta.token_endpoint, "https://auth.example.com/token");
        assert_eq!(meta.userinfo_endpoint, "https://auth.example.com/userinfo");
        assert_eq!(meta.issuer, "https://auth.example.com");
        assert_eq!(
            meta.jwks_uri,
            "https://auth.example.com/.well-known/jwks.json"
        );
    }

    #[test]
    fn test_oidc_metadata_deserialize_minimal() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "issuer": "https://auth.example.com",
            "jwks_uri": "https://auth.example.com/.well-known/jwks.json"
        }"#;
        let meta: OidcMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.issuer, "https://auth.example.com");
    }

    #[test]
    fn test_oidc_metadata_jwks_uri_rename() {
        // Verify the serde rename attribute works
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "issuer": "https://auth.example.com",
            "jwks_uri": "https://keys.example.com/jwks"
        }"#;
        let meta: OidcMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.jwks_uri, "https://keys.example.com/jwks");
    }

    #[test]
    fn test_oidc_metadata_rejects_missing_fields() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize"
        }"#;
        let result: Result<OidcMetadata, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_oidc_metadata_rejects_empty_string() {
        let result: Result<OidcMetadata, _> = serde_json::from_str("");
        assert!(result.is_err());
    }

    // ── JwtValidator ─────────────────────────────────────────

    #[test]
    fn test_jwt_validator_new() {
        let validator = JwtValidator::new("https://issuer.test", "client-123");
        assert_eq!(validator.issuer, "https://issuer.test");
        assert_eq!(validator.client_id, "client-123");
    }

    #[test]
    fn test_jwt_validator_new_empty_jwks() {
        let validator = JwtValidator::new("https://issuer.test", "client-123");
        let jwks = validator.jwks.blocking_read();
        assert!(jwks.is_empty());
    }

    #[test]
    fn test_jwt_validator_clone() {
        let v1 = JwtValidator::new("https://issuer.test", "client-123");
        let v2 = v1.clone();
        assert_eq!(v2.issuer, "https://issuer.test");
        assert_eq!(v2.client_id, "client-123");
    }

    // ── http_client ──────────────────────────────────────────

    #[test]
    fn test_http_client_exists() {
        let client = http_client();
        // Verify it's a properly configured client (not panicking)
        assert_eq!(
            std::mem::size_of_val(client),
            std::mem::size_of::<reqwest::Client>()
        );
    }

    #[test]
    fn test_http_client_is_singleton() {
        let c1 = http_client();
        let c2 = http_client();
        assert!(std::ptr::eq(c1, c2));
    }
}
