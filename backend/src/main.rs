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
mod updates;
mod workers;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tower_http::cors::CorsLayer;

use crate::auth::auth_middleware;
use crate::config::Config;
use crate::db as database;
use crate::models::*;
use crate::state::{http_client, AppState, JwtValidator, OidcMetadata, OidcStates};
use crate::workers::{auto_update_worker, state_worker, update_check_worker, CachedContainers};

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

    // Initialize SQLite database
    let config = Config::load();

    let db_pool = {
        let conn = database::init_db("data/alloy.db").expect("❌ Failed to initialize database");
        let pool: database::DbPool = Arc::new(Mutex::new(conn));
        database::init_global(pool.clone());
        pool
    };

    // Load persistent state from database
    let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = {
        let conn = db_pool.lock().await;
        Arc::new(Mutex::new(
            database::load_update_history(&conn).unwrap_or_default(),
        ))
    };
    let update_policies: Arc<Mutex<Vec<UpdatePolicy>>> = {
        let conn = db_pool.lock().await;
        Arc::new(Mutex::new(
            database::load_update_policies(&conn).unwrap_or_default(),
        ))
    };
    let settings: Arc<Mutex<Settings>> = {
        let conn = db_pool.lock().await;
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

    // Broadcast channels for SSE
    let (tx, _) = broadcast::channel(32);
    let (update_tx, _) = broadcast::channel(32);
    let (notif_tx, _) = broadcast::channel(32);

    let cached_containers: CachedContainers = Arc::new(RwLock::new(None));

    let state = AppState {
        docker: docker.clone(),
        config: config.clone(),
        tx: tx.clone(),
        update_tx: update_tx.clone(),
        notif_tx: notif_tx.clone(),
        oidc_states: Arc::new(Mutex::new(HashMap::new())),
        oidc_metadata,
        jwt_validator,
        update_history: update_history.clone(),
        update_policies: update_policies.clone(),
        cached_containers: cached_containers.clone(),
        settings: settings.clone(),
        db: db_pool.clone(),
    };

    // Spawn workers
    tokio::spawn(state_worker(
        docker.clone(),
        config.clone(),
        settings.clone(),
        tx,
        cached_containers,
    ));
    tokio::spawn(auto_update_worker(
        docker.clone(),
        config.clone(),
        settings.clone(),
        notif_tx.clone(),
        update_history.clone(),
    ));
    tokio::spawn(update_check_worker(
        docker.clone(),
        config.clone(),
        settings.clone(),
        update_policies.clone(),
        update_tx.clone(),
        notif_tx.clone(),
    ));
    tokio::spawn(oidc_states_cleanup(state.oidc_states.clone()));

    // Session secret for cookie signing (use client_secret)
    let secret_clone = config.oidc_client_secret().to_string();

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
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(
            move |headers: axum::http::HeaderMap,
                  mut req: axum::extract::Request,
                  next: axum::middleware::Next| {
                let s = secret_clone.clone();
                async move {
                    req.extensions_mut().insert(s);
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
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
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
