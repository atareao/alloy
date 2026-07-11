# AGENTS.md — Cabina de Mando Docker

## Project Overview

**Cabina de Mando Docker** (`alloy`) is a full-featured Docker management dashboard with a Rust/Axum backend and a React/TypeScript/Vite frontend. It provides real-time container monitoring, management, and automation via SSE streams, with optional OIDC authentication and Telegram/Matrix notifications.

## Tech Stack

| Layer | Technology |
|---|---|
| **Backend** | Rust (edition 2021), Axum 0.8, Tokio (full), Bollard 0.18 (Docker API) |
| **Frontend** | React, TypeScript, Vite |
| **Auth** | JWT (simple login) + OIDC (OpenID Connect discovery) |
| **Real-time** | Server-Sent Events (SSE) via `broadcast::channel` + `tokio-stream` |
| **Persistence** | JSON files (no database) — `updates_history.json`, `alerts.json`, `health_checks.json`, `schedules.json` |
| **Notifications** | Telegram Bot API, Matrix Client-Server API |
| **Build** | Multi-stage Dockerfile (Podman), `just` task runner, `vampus` versioning |
| **Linting** | `cargo clippy`, `cargo fmt` |

## Project Structure

```
/
├── AGENTS.md               # This file — project documentation for agents
├── .gitignore
├── .dockerignore
├── .justfile               # Task runner (build, lint, fmt, push, upgrade)
├── .vampus.yml             # Version management
├── Dockerfile              # Multi-stage Docker build (build context = root)
├── backend/
│   ├── Cargo.toml          # Rust dependencies
│   ├── Cargo.lock
│   ├── config.toml         # Configuration template with all options
│   ├── config.yaml         # Active runtime configuration
│   ├── src/
│   │   └── main.rs         # Entire backend (single file ~2200 lines)
│   └── target/             # Build artifacts
└── frontend/
    ├── index.html
    ├── vite.config.ts
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx         # Main React component (~1300 lines)
    │   ├── AlertsPage.tsx
    │   ├── HealthChecksPage.tsx
    │   ├── HistoryPage.tsx
    │   ├── SchedulePage.tsx
    │   ├── StacksPage.tsx
    │   └── TerminalPage.tsx
    └── dist/               # Pre-built frontend assets
```

## Architecture & Key Patterns

### 1. Single-file backend

All backend code lives in `backend/src/main.rs`. It implements the "entire app in one file" pattern — modules are separated by horizontal rule comments (`// ════════ ...`).

### 2. AppState (shared state via Axum)

```rust
struct AppState {
    docker: Docker,                                    // Bollard client
    config: Config,                                    // From config.yaml + env vars
    tx: broadcast::Sender<StateEvent>,                 // Container state SSE
    update_tx: broadcast::Sender<UpdateProgress>,       // Update progress SSE
    notif_tx: broadcast::Sender<NotifEvent>,            // Notification SSE
    oidc_states: Arc<Mutex<HashMap<String, String>>>,  // OIDC CSRF states
    oidc_metadata: Option<OidcMetadata>,               // Discovered OIDC provider
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>, // Persistent history
    alerts: Arc<Mutex<Vec<AlertConfig>>>,
    health_checks: Arc<Mutex<Vec<HealthCheck>>>,
    schedules: Arc<Mutex<Vec<ScheduleTask>>>,
    terminal_tx: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,  // Terminal sessions
}
```

Pattern: `broadcast::channel` for SSE fan-out, `Arc<Mutex<T>>` for mutable persistent state, `Arc<AppState>` shared across handlers.

### 3. SSE (Server-Sent Events)

Three SSE endpoints provide real-time updates:

- `GET /api/events` — Container state changes (every `scan_interval_secs`)
- `GET /api/updates` — Update/pull progress
- `GET /api/notifications` — Alert and notification events
- `GET /api/stats-events` — Live CPU/memory/network stats (every 3s)
- `GET /api/terminal/{name}` — Terminal output for a container

Each uses `BroadcastStream` wrapping a `broadcast::Receiver`.

### 4. Background Workers (tokio::spawn)

| Worker | Interval | Purpose |
|---|---|---|
| `state_worker` | `scan_interval_secs` (default 5s) | Polls Docker API, broadcasts container list |
| `auto_update_worker` | `auto_update_interval_hours` (default 6h) | Pulls + restarts all running containers |
| `alerts_worker` | 30s | Checks CPU/memory/status thresholds |
| `health_checks_worker` | 30s | Runs HTTP GET or ICMP ping checks |
| `scheduler_worker` | 60s | Evaluates cron expressions, executes scheduled actions |

### 5. Authentication

- **Simple JWT**: `POST /api/login` returns a JWT token (uses `jwt_secret`)
- **OIDC**: Discovers provider via `.well-known/openid-configuration`, standard auth code flow
- **Auth middleware**: Checks cookie (`session=...`), `Authorization: Bearer ...` header, or `?token=...` query param (for SSE)
- Public endpoints (no auth): `/api/stats/*`, `/api/prune`, `/api/volumes`, `/api/networks`, `/api/docker-info`

### 6. Docker API via Bollard

Connects via local Docker socket (`Docker::connect_with_local_defaults`). Key operations:

- `list_containers` — polling for state
- `inspect_container` — detailed inspection
- `create_exec` / `start_exec` — web terminal
- `stats` — live CPU/memory/network
- `restart_container`, `stop_container`, `start_container`, `remove_container` — container lifecycle
- `prune_containers`, `prune_images`, `prune_networks`, `prune_volumes` — cleanup
- `create_image` — pull images

### 7. JSON Persistence (no database)

State is persisted to JSON files loaded at startup and saved on mutation:

- `updates_history.json` — via `save_json()` in update handlers
- `alerts.json` — via `save_json()` in create/delete handlers
- `health_checks.json` — via `save_json()` in create/delete handlers
- `schedules.json` — via `save_json()` in create/delete handlers

Loaded at startup via `load_json::<T>()` generic helper.

### 8. Configuration

Loaded from `config.yaml` (YAML) with environment variable overrides:

- `HOST`, `JWT_SECRET`, `PORT`, `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`, `OIDC_REDIRECT_URL`, `SESSION_SECRET`
- `TELEGRAM_TOKEN`, `TELEGRAM_CHAT_ID`, `MATRIX_HOMESERVER`, `MATRIX_TOKEN`, `MATRIX_ROOM`

## API Routes

```
# Auth
GET  /api/auth/login              → OIDC redirect
GET  /api/auth/callback           → OIDC callback
GET  /api/auth/me                 → Session info
GET  /api/auth/logout             → Clear session
POST /api/login                   → Simple JWT login

# Containers
GET  /api/containers              → List containers
GET  /api/containers/{name}/inspect → Detailed inspect
POST /api/containers/{name}/start
POST /api/containers/{name}/stop
POST /api/containers/{name}/restart
POST /api/containers/{name}/remove

# Logs & Terminal
GET  /api/logs/{name}?tail=N      → Container logs
GET  /api/terminal/{name}         → Terminal SSE stream
POST /api/terminal/{name}/input   → Execute command

# Real-time (SSE)
GET  /api/events                  → Container state stream
GET  /api/updates                 → Update progress stream
GET  /api/notifications           → Notification stream
GET  /api/stats-events            → Live stats stream (3s interval)

# Updates
POST /api/update/{name}           → Pull + restart single container
POST /api/update-all              → Pull + restart all containers
POST /api/check-update/{name}     → Compare local vs Docker Hub

# Docker system
GET  /api/stats/{name}            → Container stats snapshot
POST /api/prune                   → Prune containers/images/networks/volumes
GET  /api/volumes                 → List volumes
GET  /api/networks                → List networks
GET  /api/docker-info             → Docker daemon info

# Stacks (docker-compose)
GET  /api/stacks                  → List compose projects
POST /api/stacks/{project}/update → Pull + recreate stack services

# History, Alerts, Health Checks, Schedule
GET/DELETE /api/history           → Update history
GET/POST/DELETE /api/alerts       → Custom alerts (cpu/memory/status)
GET/POST/DELETE /api/health-checks → HTTP/PING health checks
GET/POST/DELETE /api/schedule     → Cron-based scheduled tasks

# Config
GET  /api/config                  → Public configuration
```

## Justfile Commands

```sh
just list       # List available commands
just build      # Build Docker image via backend/Dockerfile
just push       # Push to registry
just lint       # cargo clippy (runs in backend/)
just fmt        # cargo fmt -- --check (runs in backend/)
just fmt-fix    # cargo fmt (runs in backend/)
just upgrade    # Bump version, update deps, tag, build & push
```

## Development Workflow

1. Edit `backend/src/main.rs` (backend) or `frontend/src/` (frontend)
2. Run `cd backend && cargo build` for backend
3. Build frontend: `cd frontend && npm run build`
4. Test from `backend/` with `config.yaml` (JWT secret required): `cd backend && cargo run`
5. Build production image: `just build`
6. Version bump: `just upgrade`

## Key Dependencies

| Crate | Purpose |
|---|---|
| `axum` 0.8 | HTTP framework (routes, extractors, middleware, SSE) |
| `bollard` 0.18 | Docker Engine API client |
| `tokio` 1 | Async runtime (full features) |
| `jsonwebtoken` 9 | JWT creation/validation |
| `reqwest` 0.12 | HTTP client (OIDC, Docker Hub API, health checks) |
| `serde` / `serde_json` / `serde_yaml` | Serialization |
| `tower-http` 0.6 | CORS middleware |
| `chrono` 0.4 | Timestamps and cron matching |

## Common Development Tasks

- **Add a new API route**: Add handler function + `.route()` to the `Router` chain
- **Add a new worker**: Create async fn with `tokio::spawn` in `main()`
- **Add a new config option**: Add field to `Config` struct + load logic + environment variable override
- **Add persistent state**: Add `Arc<Mutex<Vec<T>>>` to `AppState`, use `load_json`/`save_json` helpers
- **Add SSE event type**: Add struct + new `broadcast::Sender` in `AppState` + SSE route

## Notes

- This is a **single-file backend** — as it grows, consider splitting into modules (`routes/`, `workers/`, `models/`, `config/`)
- The OIDC id_token is decoded **without signature validation** (homelab-safe / insecure in production)
- Docker Compose stacks are discovered automatically via `docker compose ls --format json` — no manual path config needed
- Cron parsing is simple (5-field, no ranges `*/5` or lists `1,3,5`) — upgrade to a proper cron library if needed
