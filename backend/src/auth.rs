use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Json, Redirect, Response},
    routing::get,
    Router,
};
use chrono::{Duration, Utc};
use cookie::{Cookie, SameSite};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::json;
use std::collections::HashMap;
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
    let csrf = Uuid::new_v4().to_string();
    let state_id = Uuid::new_v4().to_string();
    oidc_states
        .lock()
        .await
        .insert(state_id.clone(), (csrf.clone(), Instant::now()));
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope=openid+profile+email&state={}&state_id={}",
        meta.authorization_endpoint,
        url_encode(config.oidc_client_id()),
        url_encode(config.oidc_redirect_url()),
        csrf, state_id,
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
    let state_id = params
        .get("state_id")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing state_id").into_response())?;
    let stored_csrf = oidc_states
        .lock()
        .await
        .remove(state_id)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid state").into_response())?;
    let stored_csrf = stored_csrf.0;
    if let Some(returned_state) = params.get("state") {
        if returned_state != &stored_csrf {
            return Err((StatusCode::BAD_REQUEST, "CSRF mismatch").into_response());
        }
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
    let jwt_claims = jwt_validator.validate_token(access_token).await.map_err(|e| {
        tracing::error!("Token validation failed: {}", e);
        (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
    })?;

    let sub = jwt_claims.sub;
    let name = jwt_claims
        .name
        .or(jwt_claims.preferred_username)
        .unwrap_or_else(|| sub.clone());
    let email = jwt_claims.email.unwrap_or_default();

    // Create session token (signed JWT cookie)
    let session_token = encode(
        &Header::default(),
        &SessionClaims {
            sub,
            name,
            email,
            exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
        },
        &EncodingKey::from_secret(config.oidc_client_secret().as_ref()),
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Session creation failed").into_response())?;

    let cookie = Cookie::build(("session", session_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
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
    let extract = |token: &str| -> Option<serde_json::Value> {
        jsonwebtoken::decode::<SessionClaims>(
            token,
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
    let cookie = Cookie::build(("session", "")).path("/").build();
    Response::builder()
        .status(302)
        .header(header::LOCATION, "/")
        .header(header::SET_COOKIE, cookie.to_string())
        .body(axum::body::Body::empty())
        .unwrap()
}

// ── Middleware ──────────────────────────────────────────────

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

    // Extract session secret from request extensions
    let secret = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_default();

    let validate = |token: &str| -> bool {
        jsonwebtoken::decode::<SessionClaims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_ref()),
            &jsonwebtoken::Validation::default(),
        )
        .is_ok()
    };

    // 1. Check session cookie
    if let Some(cookie_str) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie_str.split("; ") {
            if let Some(value) = part.strip_prefix("session=") {
                if validate(value) {
                    return Ok(next.run(req).await);
                }
            }
        }
    }
    // 2. Check Authorization Bearer header
    if let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        if validate(token) {
            return Ok(next.run(req).await);
        }
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
            if validate(token) {
                return Ok(next.run(req).await);
            }
        }
    }

    Err((StatusCode::UNAUTHORIZED, "Not authenticated").into_response())
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
    use chrono::Utc;

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

    #[test]
    fn test_session_token_roundtrip() {
        let claims = SessionClaims {
            sub: "test_user".into(),
            name: "Test".into(),
            email: "test@example.com".into(),
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
        };
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
        assert_eq!(decoded.claims.name, "Test");
        assert_eq!(decoded.claims.email, "test@example.com");
    }

    #[test]
    fn test_session_token_invalid_secret() {
        let claims = SessionClaims {
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            exp: (Utc::now() + Duration::hours(1)).timestamp() as usize,
        };
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
            sub: "user".into(),
            name: "U".into(),
            email: "u@u.com".into(),
            exp: (Utc::now() - Duration::hours(1)).timestamp() as usize,
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
}