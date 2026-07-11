<div align="center">
  <img src="./frontend/public/favicon.svg" width="64" alt="Alloy logo">

  # Alloy
  *A full-featured Docker management dashboard*

  [![Rust](https://img.shields.io/badge/Rust-1.81+-de5842?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
  [![Axum](https://img.shields.io/badge/Axum-0.8-7C3AED?style=flat-square&logo=rust&logoColor=white)](https://github.com/tokio-rs/axum)
  [![React](https://img.shields.io/badge/React-18-58C4DC?style=flat-square&logo=react&logoColor=white)](https://react.dev)
  [![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org)
  [![Docker](https://img.shields.io/badge/Docker_API-Bollard-2496ED?style=flat-square&logo=docker&logoColor=white)](https://docs.docker.com/engine/api/)
  [![Podman](https://img.shields.io/badge/Podman-Quadlet-892CA0?style=flat-square&logo=podman&logoColor=white)](https://podman.io)
  [![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)

  ⭐ If you find this useful, consider starring the repo!

  [Features](#features) • [Quick start](#quick-start) • [Configuration](#configuration) • [API](#api) • [Deployment](#deployment) • [Development](#development)

</div>

**Alloy** is a real-time Docker dashboard with a Rust/Axum backend and a React frontend. Monitor, manage, and automate your containers through a clean web interface — with live stats, web terminal, alerts, health checks, scheduled tasks, and optional OIDC authentication, all running on SSE for instant updates.

> [!NOTE]
> Alloy connects to the Docker daemon via the local socket (or Podman socket). No database required — state is persisted to JSON files.

## Features

<details open>
<summary><strong>🐳 Container Management</strong></summary>

- **Real-time monitoring** — Live container list with status, image, size, and uptime via SSE
- **Lifecycle control** — Start, stop, restart, and remove containers from the dashboard
- **Detailed inspection** — Full container metadata, mounts, networks, ports, and environment
- **Live stats** — CPU, memory, and network I/O every 3 seconds
- **Web terminal** — Interactive shell inside any running container
- **Container logs** — Tail logs with configurable line count

</details>

<details>
<summary><strong>📦 Stack & System Management</strong></summary>

- **Docker Compose stacks** — Discover and manage compose projects (pull + recreate)
- **System pruning** — Prune containers, images, networks, and volumes
- **Volume & network browsing** — List and inspect Docker volumes and networks
- **Docker info** — View daemon version, OS, drivers, and resources

</details>

<details>
<summary><strong>🔄 Automated Updates</strong></summary>

- **One-click updates** — Pull latest image and restart any container
- **Bulk update** — Update all running containers at once
- **Version diff** — Check if a newer image is available on Docker Hub
- **Auto-update worker** — Optional background daemon that checks the registry periodically
- **Update history** — Full audit trail with timestamps

</details>

<details>
<summary><strong>🔔 Alerts & Health Checks</strong></summary>

- **Custom alerts** — Define CPU, memory, and status thresholds per container
- **Health checks** — HTTP GET or ICMP ping monitors with configurable interval
- **Scheduled tasks** — Cron-based container actions (start, stop, restart, update)
- **Notifications** — Telegram and Matrix support for alert events
- **SSE notifications** — Real-time notification stream in the dashboard

</details>

<details>
<summary><strong>🔐 Authentication</strong></summary>

- **Simple JWT login** — Quick authentication with a shared secret
- **OIDC authentication** — Full OpenID Connect discovery flow (Google, Authentik, Keycloak, etc.)
- **Session management** — Cookie-based sessions with configurable secrets
- **Flexible auth** — Bearer header, cookie, or query parameter (for SSE)

</details>

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   Browser                         │
│           React + Mantine UI                      │
│           SSE streams / REST calls                │
└──────────────────────┬───────────────────────────┘
                       │ HTTP / SSE
┌──────────────────────▼───────────────────────────┐
│              Rust / Axum Backend                   │
│                                                    │
│  ┌─────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ Routes  │  │ Workers  │  │  Broadcast Channels│ │
│  │ REST    │  │ state    │  │  /events          │  │
│  │ SSE     │  │ auto-upd │  │  /updates         │  │
│  │ Auth    │  │ alerts   │  │  /notifications   │  │
│  │ Terminal│  │ health   │  │  /stats-events    │  │
│  └────┬────┘  │ schedule │  └──────────────────┘  │
│       │       └────┬─────┘                        │
│       └──────┬──────┘                             │
│              │  Bollard (Docker API)               │
└──────────────┼────────────────────────────────────┘
               │ unix socket
┌──────────────▼────────────────────────────────────┐
│              Docker / Podman Daemon                 │
│         (containers, images, volumes, networks)     │
└───────────────────────────────────────────────────┘
```

The backend is **single-file** (~2200 lines) with distinct sections separated by clear boundaries:
- **Workers** — Poll Docker API and broadcast state changes via `tokio::broadcast`
- **Routes** — Axum handlers for REST and SSE endpoints
- **Auth** — JWT + OIDC middleware with cookie/header/query parameter support
- **Persistence** — JSON files for alerts, health checks, schedules, and update history

## Quick start

### Prerequisites

- [Rust](https://www.rust-lang.org) 1.81+ (for backend development)
- [Node.js](https://nodejs.org) 20+ (for frontend development)
- Docker Engine or Podman running locally
- [Just](https://github.com/casey/just) command runner (optional, for project recipes)

### Using pre-built Docker image

```sh
# Pull and run
podman run -d \
  --name alloy \
  -p 3066:3066 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e JWT_SECRET=change-me-in-production \
  ghcr.io/atareao/alloy:latest
```

> [!TIP]
> For Podman rootless, mount `~/.local/share/containers/storage` and the Podman socket: see the [Quadlet deployment](#deployment-with-quadlet) section.

### From source

```sh
# Clone and enter project
cd /data/rust/alloy

# Build the Docker image
just build

# Or build manually (requires Rust + Node.js)
cd backend && cargo build --release
cd frontend && npm install && npm run build
```

## Configuration

Alloy is configured via a YAML file or environment variables.

### Minimal configuration

```yaml
# config.yaml
jwt_secret: "change-me-in-production"
port: 3066
```

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `JWT_SECRET` | — | **Required.** JWT signing secret |
| `PORT` | `3066` | Server listening port |
| `HOST` | `0.0.0.0` | Bind address |
| `SESSION_SECRET` | — | Session encryption secret |
| `OIDC_ISSUER_URL` | — | OIDC provider discovery URL |
| `OIDC_CLIENT_ID` | — | OIDC client ID |
| `OIDC_CLIENT_SECRET` | — | OIDC client secret |
| `OIDC_REDIRECT_URL` | — | OIDC callback URL |
| `TELEGRAM_TOKEN` | — | Telegram bot token |
| `TELEGRAM_CHAT_ID` | — | Telegram chat ID |
| `MATRIX_HOMESERVER` | — | Matrix homeserver URL |
| `MATRIX_TOKEN` | — | Matrix access token |
| `MATRIX_ROOM` | — | Matrix room ID |

## API

Alloy exposes a REST + SSE API at `http://localhost:3066/api/`.

### Container operations

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/containers` | List all containers |
| `GET` | `/api/containers/{name}/inspect` | Container details |
| `POST` | `/api/containers/{name}/start` | Start a container |
| `POST` | `/api/containers/{name}/stop` | Stop a container |
| `POST` | `/api/containers/{name}/restart` | Restart a container |
| `POST` | `/api/containers/{name}/remove` | Remove a container |

### Real-time streams (SSE)

| Endpoint | Interval | Payload |
|---|---|---|
| `/api/events` | 5s | Full container list |
| `/api/stats-events` | 3s | CPU, memory, network per container |
| `/api/updates` | On pull | Update progress per container |
| `/api/notifications` | On event | Alert and notification events |
| `/api/terminal/{name}` | Live | Container terminal output |

### System

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/stats/{name}` | Container stats snapshot |
| `POST` | `/api/prune` | Prune all (containers, images, volumes, networks) |
| `GET` | `/api/volumes` | List volumes |
| `GET` | `/api/networks` | List networks |
| `GET` | `/api/docker-info` | Docker daemon info |

### Updates

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/update/{name}` | Pull + restart a single container |
| `POST` | `/api/update-all` | Pull + restart all running containers |
| `POST` | `/api/check-update/{name}` | Compare local vs registry image |

### Alerts, Health Checks & Schedule

| Method | Endpoint | Description |
|---|---|---|
| `GET / POST / DELETE` | `/api/alerts` | CPU/memory/status alerts |
| `GET / POST / DELETE` | `/api/health-checks` | HTTP or ICMP health monitors |
| `GET / POST / DELETE` | `/api/schedule` | Cron-based scheduled tasks |
| `GET / DELETE` | `/api/history` | Update history |

### Auth

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/login` | JWT login |
| `GET` | `/api/auth/login` | OIDC login redirect |
| `GET` | `/api/auth/callback` | OIDC callback |
| `GET` | `/api/auth/me` | Current session info |
| `GET` | `/api/auth/logout` | Logout |

## Deployment

### With Quadlet (Podman systemd service)

Alloy ships with a ready-to-use Quadlet file for running as a systemd user service:

```sh
# 1. Copy the Quadlet
cp alloy.container ~/.config/containers/systemd/

# 2. Create data directory for persistent state
mkdir -p ~/.local/share/alloy/data

# 3. Reload and start
systemctl --user daemon-reload
systemctl --user start alloy

# 4. Enable at boot
systemctl --user enable alloy

# 5. View logs
journalctl --user -u alloy -f
```

The Quadlet automatically handles:
- Docker socket mounting (switch to Podman socket by uncommenting `DOCKER_HOST`)
- Persistent JSON storage (alerts, health checks, history, schedules)
- Optional custom config via `~/.config/alloy/config.yaml`
- Restart on failure

> [!WARNING]
> Remember to set `JWT_SECRET` to a strong, unique value before exposing Alloy to a network.

### Build and push custom image

```sh
just build   # Build with current version tag
just push    # Push to registry
just upgrade # Bump version, update deps, tag, build & push
```

## Development

```sh
# Just recipes
just list       # List available commands
just lint       # cargo clippy
just fmt        # cargo fmt --check
just fmt-fix    # cargo fmt
just build      # podman build

# Manual dev workflow
cd backend && cargo build     # Compile backend
cd frontend && npm run build  # Build frontend (populates dist/)
cd backend && cargo run       # Run with local config.yaml
```

The backend is a single Rust file (`backend/src/main.rs`) organized in clearly marked sections. To add a feature:

1. **New route** → Add handler + `.route()` call in the router chain
2. **New worker** → `tokio::spawn` an async loop in `main()`
3. **New config** → Add field to `Config` + env override
4. **New state** → Add `Arc<Mutex<T>>` to `AppState` + JSON persistence helpers

### Tech stack

| Layer | Technology |
|---|---|
| **Backend** | Rust, Axum 0.8, Tokio, Bollard 0.18 |
| **Frontend** | React 18, TypeScript, Vite, Mantine UI |
| **Real-time** | Server-Sent Events via `tokio::broadcast` |
| **Auth** | JWT (`jsonwebtoken`) + OIDC (OpenID Connect) |
| **Persistence** | JSON files (no database) |
| **Notifications** | Telegram Bot API, Matrix Client-Server API |
| **Deployment** | Podman, Quadlet, Docker |

---

<div align="center">
  <sub>Built with Rust + React. Powered by Docker API.</sub>
</div>
