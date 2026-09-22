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
use crate::state::{http_client, AppState, JwtValidator, OidcMetadata, OidcStates};
use crate::workers::{cleanup_worker, state_worker, update_check_worker, CachedContainers};

use axum::serve::ListenerExt;
use axum::{extract::State, response::Json, routing::get};
use bollard::Docker;

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

    let cached_containers: CachedContainers = Arc::new(RwLock::new(None));

    let progress_cache: Arc<Mutex<BatchProgress>> = Arc::new(Mutex::new(BatchProgress::default()));

    let state_notify: Arc<Notify> = Arc::new(Notify::new());

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
        progress_cache: progress_cache.clone(),
        cancel_check: Arc::new(AtomicBool::new(false)),
        state_notify: state_notify.clone(),
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
    ));
    tokio::spawn(update_check_worker(
        docker.clone(),
        settings.clone(),
        update_policies.clone(),
        update_history.clone(),
        db_pool.clone(),
        state.update_in_progress.clone(),
    ));
    tokio::spawn(oidc_states_cleanup(state.oidc_states.clone()));
    tokio::spawn(cleanup_worker(docker.clone()));

    // Session secret for cookie signing (use client_secret)
    let secret_clone = config.oidc_client_secret().to_string();
    let config_clone = config.clone();

    let app = axum::Router::new()
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
        .with_state(state);

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
    // false sense of security while SSE streams (/api/events, /api/updates,
    // /api/notifications, /api/stream) keep buffering under Nagle's algorithm.
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
    use axum::serve::{Listener, ListenerExt};
    use tokio::io::AsyncWriteExt;

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
