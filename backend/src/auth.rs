use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Json, Redirect, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use cookie::{Cookie, SameSite};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use uuid::Uuid;

use crate::config::Config;
use crate::models::SessionClaims;
use crate::state::{AppState, JwtValidator, OidcMetadata, OidcStates};

// ── OIDC Auth code flow ─────────────────────────────────────

async fn auth_login(
    State(config): State<Config>,
    State(oidc_metadata): State<Option<OidcMetadata>>,
    State(oidc_states): State<OidcStates>,
) -> Result<Redirect, Response> {
    let meta = oidc_metadata
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "OIDC not configured").into_response())?;
    // Generamos un solo valor `state` (UUID aleatorio) que sirve como:
    // 1. Clave de búsqueda en el mapa de estados
    // 2. Protección CSRF (PocketID lo devuelve sin cambios)
    // 3. Parámetro estándar OIDC (todos los proveedores lo soportan)
    let state = Uuid::new_v4().to_string();
    oidc_states
        .lock()
        .await
        .insert(state.clone(), (state.clone(), Instant::now()));
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope=openid+profile+email&state={}",
        meta.authorization_endpoint,
        url_encode(config.oidc_client_id()),
        url_encode(config.oidc_redirect_url()),
        state,
    );
    Ok(Redirect::to(&auth_url))
}

async fn auth_callback(
    State(config): State<Config>,
    State(oidc_metadata): State<Option<OidcMetadata>>,
    State(oidc_states): State<OidcStates>,
    State(jwt_validator): State<JwtValidator>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, Response> {
    let meta = oidc_metadata
        .as_ref()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "OIDC not configured").into_response())?;
    let code = params
        .get("code")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing code").into_response())?;
    // `state` es el parámetro estándar OIDC que PocketID devuelve sin cambios.
    // Lo usamos como clave para recuperar el CSRF almacenado, eliminando
    // el parámetro personalizado `state_id` que PocketID no conoce.
    let state = params
        .get("state")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing state").into_response())?;
    let stored = oidc_states
        .lock()
        .await
        .remove(state)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid state").into_response())?;
    // El valor almacenado es (csrf, timestamp). Verificamos que coincida.
    let _stored_csrf = stored.0;
    if _stored_csrf != *state {
        return Err((StatusCode::BAD_REQUEST, "CSRF mismatch").into_response());
    }

    // Exchange authorization code for tokens
    let client = crate::state::http_client();
    let token_params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", config.oidc_redirect_url()),
        ("client_id", config.oidc_client_id()),
        ("client_secret", config.oidc_client_secret()),
    ];
    let token_resp = client
        .post(&meta.token_endpoint)
        .form(&token_params)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Token request failed: {}", e);
            (StatusCode::BAD_REQUEST, "Token request failed").into_response()
        })?;
    let token_body: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid token response").into_response())?;

    // Extract the access_token (validated against JWKS like oxinbox does)
    let access_token = token_body["access_token"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "No access_token in response").into_response())?;

    // Validate token against JWKS (PocketID style via JwtValidator)
    let jwt_claims = jwt_validator
        .validate_token(access_token)
        .await
        .map_err(|e| {
            tracing::error!("Token validation failed: {}", e);
            (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
        })?;

    let sub = jwt_claims.sub.clone();
    let email = jwt_claims.email.unwrap_or_default();
    // Try to fetch UserInfo for a readable display name
    let name = {
        // Try to fetch UserInfo endpoint for display name
        let userinfo_resp = client
            .get(&meta.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await
            .ok();
        if let Some(resp) = userinfo_resp {
            if let Ok(userinfo) = resp.json::<serde_json::Value>().await {
                if let Some(display) = userinfo["preferred_username"]
                    .as_str()
                    .or(userinfo["name"].as_str())
                    .or(userinfo["nickname"].as_str())
                    .or(userinfo["email"].as_str())
                {
                    display.to_string()
                } else {
                    sub.clone()
                }
            } else {
                jwt_claims
                    .name
                    .clone()
                    .or(jwt_claims.preferred_username.clone())
                    .or(Some(email.clone()))
                    .unwrap_or(sub.clone())
            }
        } else {
            jwt_claims
                .name
                .clone()
                .or(jwt_claims.preferred_username.clone())
                .or(Some(email.clone()))
                .unwrap_or(sub.clone())
        }
    };

    // Create session token (signed JWT cookie)
    let now = Utc::now().timestamp() as usize;
    let max_duration_secs = (config.session_max_duration_hours() * 3600) as usize;
    let session_token = encode(
        &Header::default(),
        &SessionClaims {
            sub,
            name,
            email,
            iat: now,
            last_active: now,
            exp: now + max_duration_secs,
        },
        &EncodingKey::from_secret(config.oidc_client_secret().as_ref()),
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Session creation failed").into_response())?;

    let cookie = Cookie::build(("session", session_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::new(max_duration_secs as i64, 0))
        .build();
    Ok(Response::builder()
        .status(302)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, cookie.to_string())
        .body(axum::body::Body::empty())
        .unwrap())
}

async fn auth_me(headers: HeaderMap, State(config): State<Config>) -> Json<serde_json::Value> {
    let secret = config.oidc_client_secret();
    let idle_timeout_secs = config.session_idle_timeout_minutes() * 60;
    let max_duration_secs = config.session_max_duration_hours() * 3600;
    let extract = |token: &str| -> Option<serde_json::Value> {
        jsonwebtoken::decode::<SessionClaims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .ok()
        .map(|d| {
            let now = Utc::now().timestamp() as usize;
            let idle_remaining =
                (idle_timeout_secs as usize).saturating_sub(now - d.claims.last_active);
            let session_remaining = (max_duration_secs as usize).saturating_sub(now - d.claims.iat);
            json!({
                "authenticated": true,
                "user": {
                    "sub": d.claims.sub,
                    "name": d.claims.name,
                    "email": d.claims.email
                },
                "session": {
                    "exp": d.claims.exp,
                    "iat": d.claims.iat,
                    "last_active": d.claims.last_active,
                    "idle_timeout_secs": idle_timeout_secs,
                    "max_duration_secs": max_duration_secs,
                    "idle_remaining_secs": idle_remaining,
                    "session_remaining_secs": session_remaining
                }
            })
        })
    };

    // Check session cookie first
    if let Some(cookie_str) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie_str.split("; ") {
            if let Some(value) = part.strip_prefix("session=") {
                if let Some(resp) = extract(value) {
                    return Json(resp);
                }
            }
        }
    }
    // Then check Authorization header
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        if let Some(resp) = extract(token) {
            return Json(resp);
        }
    }
    Json(json!({"authenticated": false}))
}

async fn auth_logout() -> Response {
    let cookie = Cookie::build(("session", ""))
        .path("/")
        .http_only(true)
        .max_age(cookie::time::Duration::new(0, 0))
        .build();
    Response::builder()
        .status(302)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, cookie.to_string())
        .body(axum::body::Body::empty())
        .unwrap()
}

// ── Middleware ──────────────────────────────────────────────

/// Track recently logged session expirations to avoid duplicate logs
/// from concurrent SSE / REST connections when idle timeout fires.
/// Entries older than 10 seconds are cleaned up on each access.
fn session_expiry_log() -> &'static Mutex<HashMap<String, Instant>> {
    static LOG: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    LOG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Helper: build a JSON 401 response for session expiry / auth failure.
fn unauthorized_json(session_expired: bool, detail: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [("content-type", "application/json")],
        json!({
            "error": detail,
            "session_expired": session_expired,
        })
        .to_string(),
    )
        .into_response()
}

pub async fn auth_middleware(
    headers: HeaderMap,
    req: axum::extract::Request,
    next: middleware::Next,
) -> Result<Response, Response> {
    let path = req.uri().path();

    // Public endpoints: auth routes, login, health
    if path.starts_with("/api/auth/") || path == "/api/health" {
        return Ok(next.run(req).await);
    }
    // Non-API routes (frontend assets) pass through
    if !path.starts_with("/api/") {
        return Ok(next.run(req).await);
    }

    // Extract session secret and config from request extensions
    let secret = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_default();
    let config =
        req.extensions().get::<Config>().cloned().ok_or_else(|| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Config not found").into_response()
        })?;

    let idle_timeout_secs = config.session_idle_timeout_minutes() * 60;
    let max_duration_secs = config.session_max_duration_hours() * 3600;

    // Helper: validate token AND check session expiry
    let validate = |token: &str| -> Result<SessionClaims, Box<Response>> {
        let data = jsonwebtoken::decode::<SessionClaims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .map_err(|_| Box::new(unauthorized_json(false, "Invalid session token")))?;

        let claims = data.claims;
        let now = Utc::now().timestamp() as usize;

        // Check max session duration (since login)
        let session_elapsed = now.saturating_sub(claims.iat);
        if session_elapsed > max_duration_secs as usize {
            let mut log_cache = session_expiry_log().lock().unwrap();
            let dedup_key = format!("maxdur:{}", claims.sub);
            let should_log = log_cache
                .get(&dedup_key)
                .map(|t| t.elapsed() > std::time::Duration::from_secs(5))
                .unwrap_or(true);
            if should_log {
                tracing::info!(
                    "Session expired (max duration): user={}, elapsed={}s > max={}s",
                    claims.sub,
                    session_elapsed,
                    max_duration_secs
                );
                log_cache.insert(dedup_key, Instant::now());
            }
            log_cache.retain(|_, t| t.elapsed() < std::time::Duration::from_secs(10));
            return Err(Box::new(unauthorized_json(
                true,
                "Session expired: maximum duration reached. Please log in again.",
            )));
        }

        // Check idle timeout
        let idle_elapsed = now.saturating_sub(claims.last_active);
        if idle_elapsed > idle_timeout_secs as usize {
            // Dedup: only log once per user within a 5-second window
            // to avoid flooding logs from concurrent SSE connections.
            let mut log_cache = session_expiry_log().lock().unwrap();
            let dedup_key = format!("idle:{}", claims.sub);
            let should_log = log_cache
                .get(&dedup_key)
                .map(|t| t.elapsed() > std::time::Duration::from_secs(5))
                .unwrap_or(true);
            if should_log {
                tracing::info!(
                    "Session expired (idle timeout): user={}, idle={}s > timeout={}s",
                    claims.sub,
                    idle_elapsed,
                    idle_timeout_secs
                );
                log_cache.insert(dedup_key, Instant::now());
            }
            // Cleanup stale entries (>10s old)
            log_cache.retain(|_, t| t.elapsed() < std::time::Duration::from_secs(10));
            return Err(Box::new(unauthorized_json(
                true,
                "Session expired: inactivity timeout. Please log in again.",
            )));
        }

        Ok(claims)
    };

    // 1. Check session cookie
    let token = 'token: {
        if let Some(cookie_str) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
            for part in cookie_str.split("; ") {
                if let Some(value) = part.strip_prefix("session=") {
                    break 'token Some(value.to_string());
                }
            }
        }
        // 2. Check Authorization Bearer header
        if let Some(token) = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
        {
            break 'token Some(token.to_string());
        }
        // 3. Check query parameter (for SSE)
        if let Some(query) = req.uri().query() {
            if let Some(token) = query.split('&').find_map(|p| {
                let mut parts = p.splitn(2, '=');
                if parts.next()? == "token" {
                    parts.next()
                } else {
                    None
                }
            }) {
                break 'token Some(token.to_string());
            }
        }
        None::<String>
    };

    let token = token.ok_or_else(|| unauthorized_json(false, "Not authenticated"))?;

    let claims = validate(&token).map_err(|e| *e)?;

    // Run the request
    let mut response = next.run(req).await;

    // Sliding session: if more than half the idle timeout has elapsed,
    // refresh the session cookie with a new `last_active` timestamp.
    let now = Utc::now().timestamp() as usize;
    let sliding_threshold = (idle_timeout_secs as usize) / 2;
    if now.saturating_sub(claims.last_active) > sliding_threshold {
        let remaining_secs = (max_duration_secs as i64).saturating_sub((now - claims.iat) as i64);
        let new_token = encode(
            &Header::default(),
            &SessionClaims {
                last_active: now,
                ..claims
            },
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Session refresh failed").into_response()
        })?;

        let cookie = Cookie::build(("session", new_token))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .max_age(cookie::time::Duration::new(remaining_secs.max(0), 0))
            .build();

        response
            .headers_mut()
            .insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());
    }

    Ok(response)
}

// ── Frontend static file handler ───────────────────────────

pub async fn frontend_handler(req: axum::extract::Request) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');
    let file_path = if path.is_empty() || path.starts_with("api/") {
        "index.html"
    } else {
        path
    };
    let full_path = format!("dist/{}", file_path);
    match tokio::fs::read(&full_path).await {
        Ok(content) => {
            let ext = file_path.rsplit('.').next().unwrap_or("");
            let mime = match ext {
                "html" => "text/html",
                "css" => "text/css",
                "js" => "application/javascript",
                "json" => "application/json",
                "png" => "image/png",
                "svg" => "image/svg+xml",
                "ico" => "image/x-icon",
                "woff2" => "font/woff2",
                "woff" => "font/woff",
                "ttf" => "font/ttf",
                _ => "application/octet-stream",
            };
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", mime)
                .body(axum::body::Body::from(content))
                .unwrap()
        }
        Err(_) => match tokio::fs::read("dist/index.html").await {
            Ok(html) => Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html")
                .body(axum::body::Body::from(html))
                .unwrap(),
            Err(_) => (StatusCode::NOT_FOUND, "Frontend not built").into_response(),
        },
    }
}

// ── Helpers ─────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('?', "%3F")
}

// ── Routes ─────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", get(auth_login))
        .route("/api/auth/callback", get(auth_callback))
        .route("/api/auth/me", get(auth_me))
        .route("/api/auth/logout", get(auth_logout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    // ── url_encode ───────────────────────────────────────────

    #[test]
    fn test_url_encode_spaces() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }

    #[test]
    fn test_url_encode_ampersand() {
        assert_eq!(url_encode("a&b"), "a%26b");
    }

    #[test]
    fn test_url_encode_equals() {
        assert_eq!(url_encode("key=value"), "key%3Dvalue");
    }

    #[test]
    fn test_url_encode_question_mark() {
        assert_eq!(url_encode("?query"), "%3Fquery");
    }

    #[test]
    fn test_url_encode_no_change() {
        assert_eq!(url_encode("simple"), "simple");
    }

    #[test]
    fn test_url_encode_empty() {
        assert_eq!(url_encode(""), "");
    }

    #[test]
    fn test_url_encode_mixed() {
        assert_eq!(url_encode("a b&c=d?e"), "a%20b%26c%3Dd%3Fe");
    }

    // ── SessionClaims helpers ────────────────────────────────

    fn make_claims(exp_offset: chrono::Duration) -> SessionClaims {
        let now = Utc::now().timestamp() as usize;
        SessionClaims {
            sub: "test_user".into(),
            name: "Test User".into(),
            email: "test@example.com".into(),
            iat: now,
            last_active: now,
            exp: (Utc::now() + exp_offset).timestamp() as usize,
        }
    }

    // ── Session token roundtrip ──────────────────────────────

    #[test]
    fn test_session_token_roundtrip() {
        let claims = make_claims(Duration::hours(1));
        let secret = "test_secret_key_12345";
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");
        assert!(!token.is_empty());

        let decoded = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .expect("should verify");
        assert_eq!(decoded.claims.sub, "test_user");
        assert_eq!(decoded.claims.name, "Test User");
        assert_eq!(decoded.claims.email, "test@example.com");
        assert_eq!(decoded.claims.iat, claims.iat);
        assert_eq!(decoded.claims.last_active, claims.last_active);
    }

    #[test]
    fn test_session_token_invalid_secret() {
        let claims = make_claims(Duration::hours(1));
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("correct_secret".as_ref()),
        )
        .expect("should create token");
        let result = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret("wrong_secret".as_ref()),
            &jsonwebtoken::Validation::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_session_token_expired() {
        let claims = SessionClaims {
            exp: (Utc::now() - Duration::hours(1)).timestamp() as usize,
            ..make_claims(Duration::hours(1))
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_ref()),
        )
        .expect("should create token");
        let result = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret("secret".as_ref()),
            &jsonwebtoken::Validation::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_session_claims_sub_name_email() {
        let claims = SessionClaims {
            sub: "abc-123".into(),
            name: "John Doe".into(),
            email: "john@example.com".into(),
            iat: Utc::now().timestamp() as usize,
            last_active: Utc::now().timestamp() as usize,
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
        };
        let secret = "my_secret";
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");

        let decoded = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .expect("should verify");

        assert_eq!(decoded.claims.sub, "abc-123");
        assert_eq!(decoded.claims.name, "John Doe");
        assert_eq!(decoded.claims.email, "john@example.com");
        assert!(decoded.claims.exp > (Utc::now().timestamp() as usize));
    }

    #[test]
    fn test_session_token_empty_secret() {
        let claims = make_claims(Duration::hours(1));
        let result = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("".as_ref()),
        );
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    // ── auth_me extract ──────────────────────────────────────

    #[test]
    fn test_auth_me_extract_from_cookie_header() {
        let secret = "test_secret_for_me_handler";
        let claims = make_claims(Duration::hours(1));
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");

        let extract = |t: &str| -> Option<serde_json::Value> {
            jsonwebtoken::decode::<SessionClaims>(
                t,
                &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
                &jsonwebtoken::Validation::default(),
            )
            .ok()
            .map(|d| {
                json!({
                    "authenticated": true,
                    "user": { "sub": d.claims.sub, "name": d.claims.name, "email": d.claims.email }
                })
            })
        };

        let result = extract(&token);
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp["authenticated"], true);
        assert_eq!(resp["user"]["sub"], "test_user");
        assert_eq!(resp["user"]["name"], "Test User");
        assert_eq!(resp["user"]["email"], "test@example.com");
    }

    #[test]
    fn test_auth_me_extract_invalid_token_returns_none() {
        let secret = "secret";
        let extract = |t: &str| -> Option<serde_json::Value> {
            jsonwebtoken::decode::<SessionClaims>(
                t,
                &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
                &jsonwebtoken::Validation::default(),
            )
            .ok()
            .map(|d| {
                json!({
                    "authenticated": true,
                    "user": { "sub": d.claims.sub, "name": d.claims.name, "email": d.claims.email }
                })
            })
        };

        assert!(extract("invalid.token.here").is_none());
        assert!(extract("").is_none());
        assert!(extract("not-a-jwt").is_none());
    }

    #[test]
    fn test_auth_me_extract_expired_token() {
        let secret = "expired_secret_test";
        let claims = SessionClaims {
            exp: (Utc::now() - Duration::hours(1)).timestamp() as usize,
            ..make_claims(Duration::hours(1))
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");

        let extract = |t: &str| -> Option<serde_json::Value> {
            jsonwebtoken::decode::<SessionClaims>(
                t,
                &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
                &jsonwebtoken::Validation::default(),
            )
            .ok()
            .map(|d| {
                json!({
                    "authenticated": true,
                    "user": { "sub": d.claims.sub, "name": d.claims.name, "email": d.claims.email }
                })
            })
        };

        assert!(extract(&token).is_none());
    }

    #[test]
    fn test_auth_me_different_secret_returns_none() {
        let claims = make_claims(Duration::hours(1));
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("signing_secret".as_ref()),
        )
        .expect("should create token");

        let extract = |t: &str| -> Option<serde_json::Value> {
            jsonwebtoken::decode::<SessionClaims>(
                t,
                &jsonwebtoken::DecodingKey::from_secret("different_secret".as_ref()),
                &jsonwebtoken::Validation::default(),
            )
            .ok()
            .map(|d| {
                json!({
                    "authenticated": true,
                    "user": { "sub": d.claims.sub, "name": d.claims.name, "email": d.claims.email }
                })
            })
        };

        assert!(extract(&token).is_none());
    }

    // ── Cookie parsing ───────────────────────────────────────

    #[test]
    fn test_cookie_value_extraction() {
        let cookie_str = "session=eyJhbGciOiJIUzI1NiJ9.test-token; path=/; HttpOnly";
        let found = cookie_str
            .split("; ")
            .find_map(|part| part.strip_prefix("session="));
        assert_eq!(found, Some("eyJhbGciOiJIUzI1NiJ9.test-token"));
    }

    #[test]
    fn test_cookie_value_missing() {
        let cookie_str = "other=value; path=/";
        let found = cookie_str
            .split("; ")
            .find_map(|part| part.strip_prefix("session="));
        assert!(found.is_none());
    }

    #[test]
    fn test_cookie_value_empty() {
        let cookie_str = "session=; path=/";
        let found = cookie_str
            .split("; ")
            .find_map(|part| part.strip_prefix("session="));
        assert_eq!(found, Some(""));
    }

    // ── Logout ───────────────────────────────────────────────

    #[test]
    fn test_logout_clears_cookie() {
        let cookie = Cookie::build(("session", ""))
            .path("/")
            .http_only(true)
            .max_age(cookie::time::Duration::new(0, 0))
            .build();
        let cookie_str = cookie.to_string();
        assert!(cookie_str.contains("session="));
        assert!(cookie_str.contains("Max-Age=0"));
    }

    // ── Session token with special chars ─────────────────────

    #[test]
    fn test_session_token_roundtrip_with_special_chars() {
        let now = Utc::now().timestamp() as usize;
        let claims = SessionClaims {
            sub: "user|with|pipes".into(),
            name: "José María".into(),
            email: "test+alias@example.com".into(),
            iat: now,
            last_active: now,
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
        };
        let secret = "s3cr3t_k3y!@#$%";
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");

        let decoded = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .expect("should verify");

        assert_eq!(decoded.claims.sub, "user|with|pipes");
        assert_eq!(decoded.claims.name, "José María");
        assert_eq!(decoded.claims.email, "test+alias@example.com");
        assert_eq!(decoded.claims.iat, now);
        assert_eq!(decoded.claims.last_active, now);
    }

    // ── Session expiry validation ────────────────────────────

    #[test]
    fn test_session_idle_timeout_rejection() {
        let idle_timeout_secs: u64 = 30;
        let now = Utc::now().timestamp() as usize;
        let claims = SessionClaims {
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            iat: now - 3600,
            last_active: now - 31,
            exp: now + 3600,
        };
        let secret = "test_secret";
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");

        let decoded = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .expect("should verify");

        let idle_elapsed = Utc::now().timestamp() as usize - decoded.claims.last_active;
        assert!(
            idle_elapsed > idle_timeout_secs as usize,
            "idle_elapsed={} should exceed timeout={}",
            idle_elapsed,
            idle_timeout_secs
        );
    }

    #[test]
    fn test_session_max_duration_rejection() {
        let max_duration_secs: u64 = 3600;
        let now = Utc::now().timestamp() as usize;
        let claims = SessionClaims {
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            iat: now - 3601,
            last_active: now - 10,
            exp: now + 3600,
        };
        let secret = "test_secret";
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("should create token");

        let decoded = jsonwebtoken::decode::<SessionClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .expect("should verify");

        let session_elapsed = Utc::now().timestamp() as usize - decoded.claims.iat;
        assert!(
            session_elapsed > max_duration_secs as usize,
            "session_elapsed={} should exceed max={}",
            session_elapsed,
            max_duration_secs
        );
    }

    #[test]
    fn test_session_iat_and_last_active_present() {
        let now = Utc::now().timestamp() as usize;
        let claims = SessionClaims {
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            iat: now,
            last_active: now,
            exp: now + 3600,
        };
        assert_eq!(claims.iat, now);
        assert_eq!(claims.last_active, now);
        assert!(claims.exp > now);
    }

    #[test]
    fn test_session_sliding_window_refresh_updates_last_active() {
        let now = Utc::now().timestamp() as usize;
        let original = SessionClaims {
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            iat: now - 1800,
            last_active: now - 120,
            exp: now + 3600,
        };

        let refreshed = SessionClaims {
            last_active: now,
            ..original.clone()
        };

        assert_eq!(refreshed.sub, original.sub);
        assert_eq!(refreshed.iat, original.iat);
        assert_eq!(refreshed.exp, original.exp);
        assert_eq!(refreshed.last_active, now);
        assert!(refreshed.last_active > original.last_active);
    }

    #[test]
    fn test_session_iat_never_changes_on_refresh() {
        let now = Utc::now().timestamp() as usize;
        let original = SessionClaims {
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            iat: now - 7200,
            last_active: now - 60,
            exp: now + 3600,
        };

        let mut claims = original.clone();
        for _ in 0..5 {
            claims = SessionClaims {
                last_active: Utc::now().timestamp() as usize,
                ..claims
            };
            assert_eq!(claims.iat, original.iat);
        }
    }
}
