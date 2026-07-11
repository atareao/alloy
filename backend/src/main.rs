mod admin;
mod auth;
mod config;
mod containers;
mod events;
mod models;
mod notifications;
mod stacks;
mod state;
mod stats;
mod terminal;
mod updates;
mod workers;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tower_http::cors::CorsLayer;

use crate::auth::auth_middleware;
use crate::config::Config;
use crate::models::*;
use crate::state::{http_client, AppState, JwtValidator, OidcMetadata, OidcStates};
use crate::workers::{
    alerts_worker, auto_update_worker, health_checks_worker, load_json, scheduler_worker,
    state_worker, CachedContainers,
};

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

    let config = Config::load();

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
        Ok(()) => tracing::info!("✅ JWKS fetched from {}/.well-known/jwks.json", config.oidc_issuer().trim_end_matches('/')),
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

    // Persistent state
    let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> =
        Arc::new(Mutex::new(load_json(FILE_UPDATES_HISTORY)));
    let alerts: Arc<Mutex<Vec<AlertConfig>>> = Arc::new(Mutex::new({
        let mut a: Vec<AlertConfig> = load_json(FILE_ALERTS);
        if let Some(cfg_alerts) = &config.alerts {
            for ca in cfg_alerts {
                if !a.iter().any(|x| x.id == ca.id) {
                    a.push(ca.clone());
                }
            }
        }
        a
    }));
    let health_checks: Arc<Mutex<Vec<HealthCheck>>> = Arc::new(Mutex::new({
        let mut h: Vec<HealthCheck> = load_json(FILE_HEALTH_CHECKS);
        if let Some(cfg_hc) = &config.health_checks {
            for ch in cfg_hc {
                if !h.iter().any(|x| x.id == ch.id) {
                    h.push(ch.clone());
                }
            }
        }
        h
    }));
    let schedules: Arc<Mutex<Vec<ScheduleTask>>> = Arc::new(Mutex::new({
        let mut s: Vec<ScheduleTask> = load_json(FILE_SCHEDULES);
        if let Some(cfg_sched) = &config.schedule {
            for cs in cfg_sched {
                if !s.iter().any(|x| x.id == cs.id) {
                    s.push(cs.clone());
                }
            }
        }
        s
    }));

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
        alerts: alerts.clone(),
        health_checks: health_checks.clone(),
        schedules: schedules.clone(),
        terminal_tx: Arc::new(Mutex::new(HashMap::new())),
        cached_containers: cached_containers.clone(),
        prev_cpu_stats: Arc::new(std::sync::Mutex::new(HashMap::new())),
    };

    // Spawn workers
    tokio::spawn(state_worker(
        docker.clone(),
        config.clone(),
        tx,
        cached_containers,
    ));
    tokio::spawn(auto_update_worker(
        docker.clone(),
        config.clone(),
        notif_tx.clone(),
        update_history.clone(),
    ));
    tokio::spawn(alerts_worker(
        docker.clone(),
        config.clone(),
        notif_tx.clone(),
        alerts.clone(),
    ));
    tokio::spawn(health_checks_worker(
        docker.clone(),
        config.clone(),
        notif_tx.clone(),
        health_checks.clone(),
    ));
    tokio::spawn(scheduler_worker(
        docker.clone(),
        config.clone(),
        update_tx.clone(),
        notif_tx.clone(),
        schedules.clone(),
    ));
    tokio::spawn(oidc_states_cleanup(state.oidc_states.clone()));

    // Session secret for cookie signing (use client_secret)
    let secret_clone = config.oidc_client_secret().to_string();

    let app = axum::Router::new()
        .merge(auth::routes())
        .merge(admin::routes())
        .merge(containers::routes())
        .merge(events::routes())
        .merge(stats::routes())
        .merge(stacks::routes())
        .merge(terminal::routes())
        .merge(updates::routes())
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
    tracing::info!("🚀 Cabina en http://{}:{}", host, port);
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