// Los handlers axum devuelven `Result<T, Response>` por diseño (Response = 128 bytes).
// Boxear el error rompería el contrato `IntoResponse`; es el patrón habitual en apps axum.
#![allow(clippy::result_large_err)]

mod admin;
mod auth;
mod config;
mod containers;
mod db;
mod events;
mod models;
mod notifications;
mod stacks;
mod state;
#[cfg(test)]
mod test_support;
mod timezone;
mod updates;
mod workers;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, Notify, RwLock};

use crate::auth::auth_middleware;
use crate::config::Config;
use crate::db as database;
use crate::models::*;
use crate::state::{
    http_client, AppState, JwtValidator, OidcMetadata, OidcStates, RolledBackGuard, TerminalFailed,
};
use crate::updates::health_guard::restore_guards_from_history;
use crate::workers::{cleanup_worker, state_worker, update_check_worker, CachedContainers};

use axum::serve::ListenerExt;
use axum::{extract::State, response::Json, routing::get, Router};
use bollard::Docker;

/// Construye el `Router` de la aplicación a partir del estado.
///
/// Función pura: no hace `bind` ni `serve`. Extraída de `main()` (change
/// `backend-hardening`) para poder ejercitarla en tests con
/// `tower::ServiceExt::oneshot` — p. ej. las cabeceras de seguridad, que el
/// GREEN añadirá aquí como capa externa.
pub fn build_app(state: AppState) -> Router {
    let secret_clone = state.config.oidc_client_secret().to_string();
    let config_clone = state.config.clone();

    Router::new()
        .route("/api/health", get(health_h))
        .merge(auth::routes())
        .merge(admin::routes())
        .merge(config::routes())
        .merge(containers::routes())
        .merge(events::routes())
        .merge(stacks::routes())
        .merge(updates::routes())
        .merge(notifications::routes())
        .layer(axum::middleware::from_fn(
            move |headers: axum::http::HeaderMap,
                  mut req: axum::extract::Request,
                  next: axum::middleware::Next| {
                let s = secret_clone.clone();
                let c = config_clone.clone();
                async move {
                    req.extensions_mut().insert(s);
                    req.extensions_mut().insert(c);
                    auth_middleware(headers, req, next).await
                }
            },
        ))
        .fallback(auth::frontend_handler)
        // Capa MÁS externa: fija las cabeceras de seguridad en todas las
        // respuestas (API, estáticos del fallback y SSE). Se añade tras el
        // `fallback` para cubrirlo también.
        .layer(axum::middleware::from_fn(security_headers))
        .with_state(state)
}

/// Middleware que añade las cabeceras de seguridad a TODA respuesta.
///
/// `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy` y la CSP se
/// aplican siempre; `Strict-Transport-Security` solo cuando la petición llega
/// por HTTPS (esquema de la URI o `x-forwarded-proto: https`, como ocurre
/// detrás de un proxy). No altera el cuerpo ni el tipo de las respuestas, por
/// lo que no rompe la SPA ni los streams SSE.
async fn security_headers(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::{HeaderName, HeaderValue};

    let https = req.uri().scheme_str() == Some("https")
        || req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .eq_ignore_ascii_case("https")
            });

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; font-src 'self' data:; connect-src 'self'; \
             frame-ancestors 'none'; base-uri 'self'; object-src 'none'",
        ),
    );
    if https {
        headers.insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }
    response
}

async fn health_h(State(docker): State<Docker>) -> Json<serde_json::Value> {
    let docker_ok = docker.ping().await.is_ok();
    Json(serde_json::json!({
        "status": if docker_ok { "ok" } else { "degraded" },
        "docker": docker_ok,
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // jsonwebtoken 10 requires explicit CryptoProvider
    jsonwebtoken::crypto::rust_crypto::DEFAULT_PROVIDER
        .install_default()
        .expect("failed to install jsonwebtoken CryptoProvider");

    // Initialize timezone from TIMEZONE env var (default UTC)
    let tz = std::env::var("TIMEZONE").unwrap_or_default();
    timezone::init(&tz);

    // Initialize SQLite database
    let config = Config::load();

    let db_pool = database::init_db("data/alloy.db")
        .await
        .expect("❌ Failed to initialize database");

    // Load persistent state from database
    let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = {
        let obj = db_pool.get().await.expect("db pool get");
        let conn = obj.lock().unwrap();
        Arc::new(Mutex::new(
            database::load_update_history(&conn).unwrap_or_default(),
        ))
    };
    let update_policies: Arc<Mutex<Vec<UpdatePolicy>>> = {
        let obj = db_pool.get().await.expect("db pool get");
        let conn = obj.lock().unwrap();
        Arc::new(Mutex::new(
            database::load_update_policies(&conn).unwrap_or_default(),
        ))
    };
    let settings: Arc<Mutex<Settings>> = {
        let obj = db_pool.get().await.expect("db pool get");
        let conn = obj.lock().unwrap();
        Arc::new(Mutex::new(
            database::load_settings(&conn).unwrap_or_default(),
        ))
    };

    // ═══════════════════════════════════════════════════════
    // OIDC is REQUIRED — no fallback to simple JWT
    // ═══════════════════════════════════════════════════════
    if config.oidc_issuer_url.is_none()
        || config.oidc_client_id.is_none()
        || config.oidc_client_secret.is_none()
        || config.oidc_redirect_url.is_none()
    {
        tracing::error!(
            "❌ OIDC configuration required. Set all: OIDC_ISSUER_URL, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET, OIDC_REDIRECT_URL"
        );
        std::process::exit(1);
    }
    tracing::info!(
        "🔐 OIDC: issuer={}, client_id={}",
        config.oidc_issuer(),
        config.oidc_client_id()
    );

    // Discover OIDC metadata (authorization, token, userinfo endpoints)
    let well_known = format!(
        "{}/.well-known/openid-configuration",
        config.oidc_issuer().trim_end_matches('/')
    );
    let client = http_client();
    let oidc_metadata = match client.get(&well_known).send().await {
        Ok(resp) => match resp.json::<OidcMetadata>().await {
            Ok(m) => {
                tracing::info!("✅ OIDC discovery: {}", m.issuer);
                Some(m)
            }
            Err(e) => {
                tracing::error!("❌ OIDC discovery parse failed: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            tracing::error!("❌ OIDC discovery request failed: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize JWT Validator (PocketID style — fetches JWKS from issuer)
    let jwt_validator = JwtValidator::new(config.oidc_issuer(), config.oidc_client_id());
    match jwt_validator.fetch_jwks().await {
        Ok(()) => tracing::info!(
            "✅ JWKS fetched from {}/.well-known/jwks.json",
            config.oidc_issuer().trim_end_matches('/')
        ),
        Err(e) => {
            tracing::error!("❌ JWKS fetch failed: {}. OIDC will not work.", e);
            std::process::exit(1);
        }
    }

    // Docker connection
    let docker = if let Ok(host) = std::env::var("DOCKER_HOST") {
        tracing::info!("🔌 Conectando a Docker vía DOCKER_HOST: {}", host);
        if let Some(path) = host.strip_prefix("unix://") {
            bollard::Docker::connect_with_socket(path, 120, bollard::API_DEFAULT_VERSION)
        } else {
            bollard::Docker::connect_with_http(&host, 120, bollard::API_DEFAULT_VERSION)
        }
        .expect("Failed Docker via DOCKER_HOST")
    } else {
        bollard::Docker::connect_with_local_defaults().expect("Failed Docker")
    };

    // Broadcast channel for container state events
    let (tx, _) = broadcast::channel(128);
    // Broadcast channels for the single SSE multiplexed stream (GET /api/stream)
    let (progress_tx, _) = broadcast::channel::<UpdateProgress>(128);
    let (notif_tx, _) = broadcast::channel::<NotifEvent>(128);
    let (container_event_tx, _) = broadcast::channel::<ContainerEvent>(128);

    let cached_containers: CachedContainers = Arc::new(RwLock::new(None));

    let progress_cache: Arc<Mutex<BatchProgress>> = Arc::new(Mutex::new(BatchProgress::default()));

    let state_notify: Arc<Notify> = Arc::new(Notify::new());

    // Reconstruir los guards de la guarda de arranque desde el historial
    // persistido (change: persist-terminal-update-state): un reinicio NO debe
    // perder la supresión terminal (`failed-stopped`, D9) ni la memoria
    // anti-churn de rollback (D10).
    //
    // Se restaura con **poda por presencia** (contenedores presentes en el
    // daemon) y NO se usa la tabla `containers`/`has_update` como fuente de
    // elegibilidad: esa tabla no se rellena en producción. Las supresiones
    // obsoletas (sin update pendiente) las reconcilia el scheduler en su primer
    // ciclo con el `has_update` en vivo. Ante cualquier fallo de lectura es
    // **fail-open** (guards vacíos) y emite `tracing::warn!`, para no bloquear
    // updates por un error transitorio ni dejar la persistencia muda.
    let (terminal_failed, rolled_back) = {
        // 1. Último estado de historial por contenedor (independiente del
        // LIMIT 100 de la lista en memoria).
        let latest = {
            let obj = db_pool.get().await.expect("db pool get");
            let conn = obj.lock().unwrap();
            match database::load_latest_history_per_container(&conn) {
                Ok(entries) => entries,
                Err(e) => {
                    tracing::warn!(
                        "no se pudo reconstruir guards desde historial: {e}; arrancando con guards vacíos"
                    );
                    Vec::new()
                }
            }
        };
        // 2. Contenedores presentes en el daemon. Se listan TODOS (`all: true`)
        // porque un contenedor `failed-stopped` está parado pero sigue existiendo.
        let present: HashSet<String> = match docker
            .list_containers(Some(bollard::container::ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
        {
            Ok(list) => list
                .iter()
                .filter_map(|c| {
                    c.names
                        .as_ref()
                        .and_then(|n| n.first())
                        .map(|n| strip_name(n))
                })
                .collect(),
            Err(e) => {
                tracing::warn!(
                    "no se pudo listar contenedores para restaurar guards: {e}; no se restaura supresión"
                );
                HashSet::new()
            }
        };
        restore_guards_from_history(&latest, &present)
    };

    let state = AppState {
        docker: docker.clone(),
        config: config.clone(),
        tx: tx.clone(),
        oidc_states: Arc::new(Mutex::new(HashMap::new())),
        oidc_metadata,
        jwt_validator,
        update_history: update_history.clone(),
        update_policies: update_policies.clone(),
        cached_containers: cached_containers.clone(),
        settings: settings.clone(),
        db: db_pool.clone(),
        update_in_progress: Arc::new(Mutex::new(HashSet::new())),
        terminal_failed: TerminalFailed(Arc::new(Mutex::new(terminal_failed))),
        rolled_back: RolledBackGuard(Arc::new(Mutex::new(rolled_back))),
        progress_cache: progress_cache.clone(),
        cancel_check: Arc::new(AtomicBool::new(false)),
        state_notify: state_notify.clone(),
        progress_tx: progress_tx.clone(),
        notif_tx: notif_tx.clone(),
        container_event_tx: container_event_tx.clone(),
    };

    // Spawn workers
    tokio::spawn(state_worker(
        docker.clone(),
        settings.clone(),
        update_policies.clone(),
        tx.clone(),
        cached_containers,
        db_pool.clone(),
        state.update_in_progress.clone(),
        state_notify,
        notif_tx.clone(),
        container_event_tx.clone(),
    ));
    tokio::spawn(update_check_worker(
        docker.clone(),
        settings.clone(),
        update_policies.clone(),
        update_history.clone(),
        db_pool.clone(),
        state.update_in_progress.clone(),
        state.terminal_failed.clone(),
        state.rolled_back.clone(),
    ));
    tokio::spawn(oidc_states_cleanup(state.oidc_states.clone()));
    tokio::spawn(cleanup_worker(docker.clone()));

    let app = build_app(state);

    let port = config.port();
    let host = config.host();
    tracing::info!("🚀 Alloy en http://{}:{}", host, port);
    let addr = if host == "0.0.0.0" {
        format!("[::]:{}", port)
    } else {
        format!("{}:{}", host, port)
    };
    let addr: std::net::SocketAddr = addr
        .parse()
        .unwrap_or_else(|_| format!("[::]:{}", port).parse().unwrap());
    let socket = match addr {
        std::net::SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6().unwrap(),
        std::net::SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4().unwrap(),
    };
    // NOTE: TCP_NODELAY is intentionally NOT set on this listening `TcpSocket`.
    // On Linux, socket options set on a listening socket do not propagate to
    // the connections it accepts via accept(2). Setting it here would give a
    // false sense of security while SSE streams (/api/stream and the container
    // log endpoints) keep buffering under Nagle's algorithm.
    // Instead, TCP_NODELAY is applied per accepted connection below via
    // `axum::serve::ListenerExt::tap_io`.
    socket.bind(addr).unwrap();
    let listener = socket
        .listen(1024)
        .unwrap()
        .tap_io(|tcp_stream: &mut tokio::net::TcpStream| {
            if let Err(e) = tcp_stream.set_nodelay(true) {
                tracing::trace!("failed to set TCP_NODELAY on incoming connection: {e}");
            }
        });
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("ctrl_c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => { tracing::info!("🛑 SIGINT received, shutting down..."); }
        _ = terminate => { tracing::info!("🛑 SIGTERM received, shutting down..."); }
    }
}

/// Clean up expired OIDC CSRF states every 5 minutes
async fn oidc_states_cleanup(oidc_states: OidcStates) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(300));
    loop {
        tick.tick().await;
        let mut states = oidc_states.lock().await;
        let before = states.len();
        states.retain(|_, (_, ts)| ts.elapsed() < std::time::Duration::from_secs(600));
        let removed = before - states.len();
        if removed > 0 {
            tracing::info!("🧹 Cleaned {} expired OIDC CSRF states", removed);
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::serve::{Listener, ListenerExt};
    use tokio::io::AsyncWriteExt;
    use tower::ServiceExt;

    use super::build_app;
    use crate::db;
    use crate::models::SessionClaims;

    /// Config con los cuatro campos OIDC mínimos (necesarios para `build_app`,
    /// que deriva el secreto de firma de sesión de `oidc_client_secret`).
    fn hardening_test_config() -> crate::config::Config {
        crate::config::Config {
            oidc_issuer_url: Some("http://localhost".into()),
            oidc_client_id: Some("test-client".into()),
            oidc_client_secret: Some("test-secret".into()),
            oidc_redirect_url: Some("http://localhost/callback".into()),
            ..Default::default()
        }
    }

    /// Session token válido firmado con `secret`, para atravesar `auth_middleware`.
    fn hardening_session_token(secret: &str) -> String {
        use chrono::Utc;
        use jsonwebtoken::{encode, EncodingKey, Header};
        let now = Utc::now().timestamp() as usize;
        let claims = SessionClaims {
            sub: "test-user".into(),
            name: "Test User".into(),
            email: "test@example.com".into(),
            iat: now,
            last_active: now,
            exp: now + 3600,
            purpose: None,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .expect("session token")
    }

    /// RED (backend-hardening 1.1): un handler que usa la pool NO debe panickear
    /// cuando la pool está cerrada; debe devolver HTTP 500 con JSON `{"error": ...}`.
    ///
    /// **Detección del RED**: hoy `update_config_h` hace `db_pool.get().await.unwrap()`
    /// y su firma es infalible (`Json<T>`), de modo que el `unwrap` panickea al pollear
    /// el handler dentro del router; el panic se propaga por el futuro de `oneshot`
    /// (que se `await`ea en la propia tarea del test) y el test falla **por panic**,
    /// no por aserción. Tras el GREEN (`AppError::from_pool` + firma
    /// `Result<Json<T>, AppError>`) el panic desaparece y se cumplen las aserciones
    /// de 500 + `{"error": ...}`.
    #[tokio::test]
    async fn closed_pool_handler_returns_500_json_without_panicking() {
        let pool = db::test_pool();
        // Cierra la pool: cualquier `db_pool.get()` posterior devuelve
        // `PoolError::Closed` en lugar de una conexión.
        pool.close();
        let app = build_app(crate::test_support::test_app_state(
            hardening_test_config(),
            pool,
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/config")
                    .header("content-type", "application/json")
                    .header(
                        "cookie",
                        format!("session={}", hardening_session_token("test-secret")),
                    )
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("el handler no debe panickear con la pool cerrada");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
        assert!(
            json.get("error").is_some(),
            "el cuerpo 500 debe ser {{\"error\": ...}}: {json}"
        );
    }

    /// RED (backend-hardening 1.3): toda respuesta MUST llevar las cabeceras de
    /// seguridad. Hoy `build_app` no las añade, así que la aserción falla.
    #[tokio::test]
    async fn security_headers_present_on_response() {
        let app = build_app(crate::test_support::test_app_state(
            hardening_test_config(),
            db::test_pool(),
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/login")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("oneshot");

        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["x-frame-options"], "DENY");
        assert_eq!(
            response.headers()["referrer-policy"],
            "strict-origin-when-cross-origin"
        );
        let csp = response.headers()["content-security-policy"]
            .to_str()
            .expect("CSP is ASCII");
        assert!(
            csp.contains("style-src 'self' 'unsafe-inline'"),
            "CSP debe permitir los estilos inline de AntD (style-src 'self' 'unsafe-inline'): {csp}"
        );
        assert!(
            csp.contains("script-src 'self'"),
            "CSP debe restringir los scripts a 'self' (script-src 'self'): {csp}"
        );
    }

    /// RED (backend-hardening 1.3): las cabeceras de seguridad MUST cubrir también
    /// las respuestas servidas por el `fallback` (estático/SPA), no solo la API.
    ///
    /// Una ruta que no empieza por `/api/` atraviesa `auth_middleware` sin exigir
    /// sesión y la sirve `frontend_handler`. Aunque `dist/` no exista en el entorno
    /// de tests (fallback a 404 "Frontend not built"), la capa MÁS externa
    /// `security_headers` MUST añadir igualmente las cabeceras a esa respuesta.
    #[tokio::test]
    async fn security_headers_present_on_static_spa_fallback() {
        let app = build_app(crate::test_support::test_app_state(
            hardening_test_config(),
            db::test_pool(),
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/some-spa-route")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("oneshot");

        // El fallback estático/SPA es público: no debe exigir sesión.
        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "el fallback estático/SPA no debe exigir autenticación"
        );
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["x-frame-options"], "DENY");
        assert_eq!(
            response.headers()["referrer-policy"],
            "strict-origin-when-cross-origin"
        );
    }

    /// RED (backend-hardening 1.3): cuando la petición llega por HTTPS
    /// (`x-forwarded-proto: https`) la respuesta MUST incluir HSTS. Hoy no hay
    /// middleware de cabeceras, así que falta.
    #[tokio::test]
    async fn hsts_header_present_when_https_forwarded() {
        let app = build_app(crate::test_support::test_app_state(
            hardening_test_config(),
            db::test_pool(),
        ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/login")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("oneshot");

        assert!(
            response.headers().contains_key("strict-transport-security"),
            "debe incluir Strict-Transport-Security cuando x-forwarded-proto: https"
        );
    }

    /// RED->GREEN test for sse-nodelay-fix.
    ///
    /// Verifies that connections *accepted* through a `TcpListener` wrapped
    /// with `.tap_io(...)` have `TCP_NODELAY` enabled, which is exactly the
    /// pattern applied in `main()` to fix SSE buffering caused by Nagle's
    /// algorithm on accepted sockets (nodelay on the listening `TcpSocket`
    /// does NOT propagate to accepted connections on Linux).
    #[tokio::test]
    async fn accepted_connections_have_tcp_nodelay_via_tap_io() {
        let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let local_addr = tcp_listener.local_addr().expect("local_addr");

        let (nodelay_tx, nodelay_rx) = tokio::sync::oneshot::channel::<bool>();
        let mut nodelay_tx = Some(nodelay_tx);

        // Same pattern as production: wrap the listener with tap_io and set
        // TCP_NODELAY on each accepted `TcpStream`.
        let mut tapped_listener =
            tcp_listener.tap_io(move |tcp_stream: &mut tokio::net::TcpStream| {
                if let Err(e) = tcp_stream.set_nodelay(true) {
                    tracing::trace!("failed to set TCP_NODELAY on incoming connection: {e}");
                }
                if let Some(tx) = nodelay_tx.take() {
                    let _ = tx.send(tcp_stream.nodelay().unwrap_or(false));
                }
            });

        let accept_task = tokio::spawn(async move {
            let (_io, _addr) = tapped_listener.accept().await;
        });

        // Connect a client to trigger the accept() above.
        let mut client = tokio::net::TcpStream::connect(local_addr)
            .await
            .expect("client connect");
        client.write_all(b"ping").await.ok();

        let nodelay_enabled = nodelay_rx
            .await
            .expect("tap_fn should report nodelay state");
        accept_task.await.expect("accept task should finish");

        assert!(
            nodelay_enabled,
            "TCP_NODELAY must be enabled on the accepted connection via tap_io, \
             not only on the listening socket (which does not propagate it on Linux)"
        );
    }
}
