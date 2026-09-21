# AGENT DIRECTIVES: OPENSPEC (SDD) + TDD WORKFLOW

## I. CORE PRINCIPLES & GOALS

- **Phase 0 — Legacy Support:** If modifying existing code without specs or tests, establish a baseline spec and characterization tests before introducing changes.
- **Phase 1 — SDD (OpenSpec):** No new code or tests may be written before a spec change proposal exists in `openspec/changes/<feature>/` and is approved by the user.
- **Phase 2 — TDD (Red-Green-Refactor):** Once the spec is approved, code MUST be developed strictly test-first using terminal commands.
- **Strict Verification:** Always run CLI test suites using terminal tools. Never assume code or tests pass/fail without CLI confirmation.

> **🚫 NO-SKIP CLAUSE**: Ninguna instrucción del usuario — incluyendo "adelante", "ejecuta", "procede", "go ahead", "sounds good", "looks good" o cualquier otra variante — invalida los pasos SDD → TDD. El agente debe completar SDD (generar change proposal + esperar aprobación explícita del spec) y TDD (RED → GREEN → REFACTOR) antes de escribir código de aplicación. Si el usuario da una orden ambigua, el agente DEBE responder: *"¿Quieres que genere el change proposal en openspec/changes/ primero?"* antes de implementar.
> 
> **Consecuencia**: Cualquier código escrito sin seguir SDD + TDD se considera una violación del proceso. El agente debe detenerse, crear el spec retroactivo, y rehacer el trabajo con TDD.

> **🔁 FOLLOW-UP CLAUSE**: Cada ronda de ajustes, correcciones o refinamientos sobre un cambio ya archivado requiere SU PROPIO change proposal (ej: `ui-fixes-v2`, `auth-refactor-round-2`). Un cambio archivado NO autoriza modificaciones directas al código. El agente DEBE crear un nuevo change proposal incremental antes de tocar cualquier archivo fuente.

> **📁 SPEC FILE NAMING**: Los archivos spec delta dentro de `openspec/changes/<feature>/specs/<capability>/` DEBEN llamarse `spec.md`. El nombre `layout.md`, `auth.md`, etc. NO es válido — el validador de `openspec archive` los ignora. La estructura correcta es: `specs/ui/layout/spec.md` (no `specs/ui/layout.md`).

> **✅ APPROVED MARKER**: Tras la aprobación explícita del usuario, el agente DEBE crear un archivo `.approved` vacío en el directorio del change proposal (`touch openspec/changes/<feature>/.approved`). Esto evita que `require-proposal.sh` siga mostrando el warning "sin aprobación explícita" en ejecuciones posteriores.

> **🧹 POST-ARCHIVE CLEANUP**: Después de ejecutar `openspec archive <feature> --yes`, el agente DEBE verificar que el directorio `openspec/changes/<feature>/` fue eliminado. Si el archive no lo limpia automáticamente, el agente DEBE eliminarlo manualmente (`rm -rf openspec/changes/<feature>/`). De lo contrario, `require-proposal.sh` lo detectará como proposal activo fantasma.

> **⚠️ ARCHIVE PRE-FLIGHT**: Antes de archivar, verificar que la ruta destino en `openspec/specs/` no tenga ya un spec que entre en conflicto. Si existe un spec en `specs/ui/layout/spec.md` y el archive va a crear `specs/ui/spec.md`, hay que mover/renombrar primero para evitar duplicación.

> **⚠️ PRE-FLIGHT OBLIGATORIO**: Antes de CUALQUIER tool call que lea o escriba archivos de código (`backend/src/*.rs`, `frontend/src/*.{tsx,ts}`), el agente DEBE ejecutar `source openspec/require-proposal.sh`. Si falla, el agente DEBE detenerse y crear un change proposal primero. Esta validación es innegociable.

---

## II. EXECUTION WORKFLOW

### Phase 0: Legacy Code Preparation (Conditional)

*Execute this phase ONLY if modifying an existing module/file that lacks OpenSpec documentation or tests.*

1. **Characterization Spec (As-Is):**
   - Inspect the target file/module.
   - Generate a baseline spec in `openspec/specs/<module>/spec.md` reflecting current behavior.
2. **Characterization Tests:**
   - Write Rust (`#[test]`) or React/TS (`vitest` / `@testing-library/react`) tests matching current behavior.
   - Run tests via CLI (`cargo test` or `npx vitest run`) to confirm all pass in **GREEN**.

### Phase 1: SDD Protocol (OpenSpec)

When the user requests a new feature, bug fix, or refactor:

0. **🔒 PRE-FLIGHT:**
   - Run `source openspec/validate-workflow.sh` as the **first step** before anything else.
   - If it fails, STOP. Create the change proposal first.

1. **Create the Change Proposal:****
   - Execute CLI command: `openspec change <feature-name>`
2. **Draft Specifications:**
   - Populate `openspec/changes/<feature-name>/proposal.md` with intent, scope, and impact.
   - Create spec deltas in `openspec/changes/<feature-name>/specs/<module>/spec.md`.
   - Ensure the spec includes:
     - **Contracts:** Rust types/structs/enums, TypeScript interfaces/props, API endpoints, or function signatures.
     - **Scenarios (BDD style):** Detailed `Given / When / Then` clauses for happy path, error cases, and edge cases.
   - Populate `openspec/changes/<feature-name>/tasks.md` with the TDD task checklist.
3. **STOP & WAIT FOR APPROVAL:**
   - Present the created specification to the user.
   - **DO NOT** write application code or new tests until the user explicitly approves the spec.
   - **Tras aprobación explícita**: crear el archivo `.approved`:
     ```bash
     touch openspec/changes/<feature-name>/.approved
     ```

### Phase 2: TDD Protocol (Red-Green-Refactor)

Once the user approves the spec (e.g., "Approved", "Looks good", "Proceed with TDD"):

1. **RED (Write Failing Tests):**
   - Read the `Given / When / Then` scenarios in `openspec/changes/<feature-name>/specs/`.
   - Write tests in Rust or React/TypeScript corresponding to those scenarios.
   - Execute CLI tests (`cargo test` or `npx vitest run`).
   - **Verify:** Confirm test failure for the new functionality while any legacy tests remain **GREEN**.
2. **GREEN (Minimal Implementation):**
   - Write the absolute minimum code necessary to satisfy the failing tests.
   - Execute CLI tests (`cargo test` or `npx vitest run`).
   - Run type checks (`cargo check` or `npx tsc -b`).
   - **Verify:** Confirm all tests pass (100% green) and no compilation/type errors exist.
3. **REFACTOR (Clean & Consolidate):**
   - Clean up code formatting, types, and structure without altering behavior.
   - Run linters (`cargo clippy -- -D warnings` / `npm run lint`).
   - Re-run test suites via CLI to guarantee no regressions.
4. **CONSOLIDATE & ARCHIVE:**
   - Mark completed items in `tasks.md`.
   - **PRE-FLIGHT**: Verificar que la ruta destino en `openspec/specs/` no tenga ya un spec que entre en conflicto (ej: `specs/ui/spec.md` vs `specs/ui/layout/spec.md`).
   - Once all scenarios pass, run `openspec archive <feature-name> --yes` to merge the delta into `openspec/specs/`.
   - **POST-ARCHIVE CLEANUP**: Verificar que `openspec/changes/<feature-name>/` fue eliminado. Si no, eliminarlo manualmente:
     ```bash
     rm -rf openspec/changes/<feature-name>/
     ```

---

## III. PROJECT CONFIGURATION & CONVENTIONS

### Stack Commands

#### Backend: Rust
- **Test Runner:** `cargo test` (or `cargo nextest run` if available).
- **Type Checking & Linting:** `cargo check` and `cargo clippy -- -D warnings` (enforce zero warnings).
- **Formatting:** `cargo fmt --check`
- **Conventions:**
  - Structs and types placed in domain modules or `src/models/`.
  - Unit tests placed in the same file under `#[cfg(test)]`.
  - Integration and API tests placed in `tests/`.

#### Frontend: React + TypeScript
- **Test Runner:** `npx vitest run` or `npm test -- --watch=false` (single-pass execution).
- **Type Checking:** `npx tsc -b` (mandatory during GREEN/REFACTOR steps).
- **Linting & Formatting:** `npm run lint` / `npx eslint .`
- **Conventions:**
  - Components in `src/components/`, hooks in `src/hooks/`.
  - Component tests colocated as `Component.test.tsx` using `@testing-library/react`.
  - User-centric testing behavior using `@testing-library/user-event` instead of implementation details.

### Custom Repository Rules

- Insert here any specific business logic, database conventions, or custom architectural rules unique to this project.

---

## IV. RESPONSE FORMAT & STATUS MESSAGES

Always prefix your progress updates with the current status tag:

```text
[LEGACY - INSPECT] Creating baseline spec & characterization tests.
[OPENSPEC - DRAFT] Generating change proposal in openspec/changes/...
[OPENSPEC - WAITING] Spec generated. Awaiting user review and approval.
[TDD - RED] Creating tests for scenario <Name> -> Running CLI tests.
[TDD - GREEN] Implementing minimal code -> Running CLI tests & type checks.
[TDD - REFACTOR] Refactoring code -> Running Clippy/ESLint & tests.
[OPENSPEC - ARCHIVE] Archiving change into openspec/specs/.
```


---

## V. CURRENT PROJECT STATE


### Project Overview

**Alloy** is a full-featured Docker management dashboard with a Rust/Axum backend and a React/TypeScript/Vite frontend. Provides real-time container monitoring, management, and automation via SSE streams, with **mandatory OIDC authentication** (PocketID-style JWKS validation) and Telegram/Matrix notifications.

### Tech Stack

| Layer | Technology |
|---|---|
| **Backend** | Rust (edition 2021), Axum 0.8, Tokio (full), Bollard 0.18 (Docker API) |
| **Frontend** | React, TypeScript, Vite, Ant Design v6, Vitest |
| **Auth** | OIDC obligatorio (no fallback JWT simple). Validación de tokens contra JWKS vía `{issuer}/.well-known/jwks.json` |
| **Real-time** | Server-Sent Events (SSE) via `broadcast::channel` + `tokio-stream` |
| **Persistence** | JSON files (no database) — `data/updates_history.json`, `data/alerts.json`, `data/schedules.json`, `data/settings.json` |
| **Notifications** | Telegram Bot API, Matrix Client-Server API |
| **Build** | Multi-stage Dockerfile (Podman), `just` task runner, `vampus` versioning |
| **Linting** | `cargo clippy -- -D warnings`, `cargo fmt -- --check` |

### Project Structure

```
/
├── AGENTS.md               # This file — project documentation for agents
├── .gitignore
├── .dockerignore
├── .justfile               # Task runner (build, lint, fmt, check, gitflow, upgrade)
├── .vampus.yml             # Version management (current: 0.6.0)
├── Dockerfile              # Multi-stage Docker build (build context = root)
├── data/                   # Persistent JSON files (runtime)
├── backend/
│   ├── Cargo.toml          # Rust dependencies
│   ├── Cargo.lock
│   ├── config.yaml         # Active runtime configuration
│   ├── src/
│   │   ├── main.rs         # (289) Entry point — startup, workers spawn, router
│   │   ├── admin.rs        #  (96) Admin handlers (alerts, settings)
│   │   ├── auth.rs         # (480) OIDC auth code flow, middleware, frontend SPA fallback
│   │   ├── config.rs       # (338) Config struct, YAML load, env override, Podman secrets
│   │   ├── containers.rs   # (410) Container CRUD, inspect, fetch, pull
│   │   ├── events.rs       #  (62) SSE event stream handler
│   │   ├── models.rs       # (402) All data types, constants, AppError
│   │   ├── notifications.rs# (135) Telegram & Matrix notification dispatchers
│   │   ├── persistence.rs  # (153) JSON load/save helpers
│   │   ├── stacks.rs       # (374) Docker Compose stack management
│   │   ├── state.rs        # (216) AppState, JwtValidator, OidcMetadata, FromRef impls
│   │   ├── updates.rs      # (524) Image pull, update, digest compare, version check
│   │   └── workers.rs      # (756) Background workers: state, auto-update, alerts, scheduler
│   └── target/             # Build artifacts
└── frontend/
    ├── index.html
    ├── vite.config.ts
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx           # Main app with tabs: Dashboard, History, Alerts, Schedule, Config
    │   ├── api.ts            # (19) API helper functions
    │   ├── types.ts          # TypeScript interfaces
    │   ├── useSSE.ts         # SSE hook for real-time events
    │   ├── AlertsPage.tsx
    │   ├── HistoryPage.tsx
    │   ├── SchedulePage.tsx
    │   ├── components/
    │   │   ├── DashboardPage.tsx
    │   │   ├── ConfigPage.tsx
    │   │   ├── LoginScreen.tsx
    │   │   ├── PolicyActionButton.tsx
    │   │   ├── ErrorBoundary.tsx
    │   │   └── NotifToast.tsx
    │   ├── api.test.ts
    │   ├── ErrorBoundary.test.tsx
    │   ├── LoginScreen.test.tsx
    │   └── NotifToast.test.tsx
    └── dist/                # Pre-built frontend assets
```

### Architecture & Key Patterns

#### 1. Modular backend (13 módulos)

El backend está organizado en 13 módulos (~4.235 líneas totales). Cada módulo tiene una responsabilidad clara:

| Módulo | Líneas | Responsabilidad |
|---|---|---|
| `workers.rs` | 756 | Workers asíncronos: estado Docker, auto-update, alertas, scheduler |
| `updates.rs` | 524 | Pull/update de imágenes, comparación de versiones |
| `auth.rs` | 480 | OIDC auth code flow, middleware de sesión, frontend SPA fallback |
| `containers.rs` | 410 | CRUD de contenedores, inspect, fetch, pull |
| `models.rs` | 402 | Tipos, constantes, AppError, tests |
| `stacks.rs` | 374 | Gestión de stacks Docker Compose |
| `config.rs` | 338 | Config (YAML + env vars + Podman secrets) |
| `main.rs` | 289 | Entry point, startup, workers, router |
| `state.rs` | 216 | AppState, JwtValidator, OidcMetadata, FromRef impls |
| `persistence.rs` | 153 | Helpers genéricos load/save JSON |
| `notifications.rs` | 135 | Dispatchers Telegram y Matrix |
| `admin.rs` | 96 | Handlers de admin (alerts, settings) |
| `events.rs` | 62 | Handler SSE de eventos de estado |

#### 2. AppState (shared state via Axum)

```rust
struct AppState {
    docker: Docker,                                    // Bollard client
    config: Config,                                    // From config.yaml + env vars
    tx: broadcast::Sender<StateEvent>,                 // Container state SSE
    update_tx: broadcast::Sender<UpdateProgress>,       // Update progress SSE
    notif_tx: broadcast::Sender<NotifEvent>,            // Notification SSE
    oidc_states: OidcStates,                           // OIDC CSRF states (con timestamp)
    oidc_metadata: Option<OidcMetadata>,               // Descubierto via OIDC discovery
    jwt_validator: JwtValidator,                       // JWKS-based token validation
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    alerts: Arc<Mutex<Vec<AlertConfig>>>,
    schedules: Arc<Mutex<Vec<ScheduleTask>>>,
    cached_containers: CachedContainers,               // Cache de contenedores (Arc<RwLock<Option<Vec<ContainerInfo>>>>)
    settings: Arc<Mutex<Settings>>,                    // Settings dinámicos (auto-update, notificaciones)
}
```

Pattern: `broadcast::channel` para SSE fan-out, `Arc<Mutex<T>>` para estado mutable persistente, `Arc<AppState>` compartido vía `axum::extract::FromRef` entre handlers.

#### 3. SSE (Server-Sent Events)

Cuatro SSE endpoints proveen actualizaciones en tiempo real:

- `GET /api/events` — Cambios de estado de contenedores (Docker Events API)
- `GET /api/updates` — Progreso de pull/update de imágenes
- `GET /api/notifications` — Eventos de alertas y notificaciones

Cada uno usa `BroadcastStream` wrapping un `broadcast::Receiver`. La autenticación SSE se hace vía cookie de sesión (httponly).

#### 4. Background Workers (tokio::spawn)

| Worker | Intervalo | Propósito |
|---|---|---|
| `state_worker` | Docker Events API + fallback 30s | Escucha eventos Docker (start/stop/die/etc.), refresca lista de contenedores |
| `auto_update_worker` | `auto_update_interval_hours` (default 6h) | Pull + restart de contenedores con auto-update |
| `alerts_worker` | 30s | Monitorea cambios de estado de contenedores (running→exited→running) |
| `scheduler_worker` | 60s | Evalúa expresiones cron, ejecuta acciones programadas |
| `oidc_states_cleanup` | 5 min | Limpia estados OIDC CSRF expirados (>10 min) |

#### 5. Authentication (OIDC obligatorio)

- **No hay JWT simple** — no existe `POST /api/login`
- **OIDC es obligatorio**: se requieren `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`, `OIDC_REDIRECT_URL`
- **Discovery**: obtiene metadata via `{issuer}/.well-known/openid-configuration`
- **JWKS validation**: descarga claves de `{issuer}/.well-known/jwks.json` y valida tokens con RSA256
- **Auth code flow**: `GET /api/auth/login` → redirect al provider → callback → sesión vía cookie firmada
- **Auth middleware**: chequea cookie `session=...` (firmada con `oidc_client_secret`), `Authorization: Bearer ...` header, o `?token=...` query param (para SSE)
- **JwtValidator** en `state.rs`: estilo PocketID/oxinbox, con `fetch_jwks()` al startup y auto-fetch en primer uso

#### 6. Docker API via Bollard

Conecta via socket local (`Docker::connect_with_local_defaults`) o `DOCKER_HOST` env var. Key operations:

- `list_containers` — polling para estado
- `inspect_container` — inspección detallada
- `stats` — stats de contenedor
- `restart_container`, `stop_container`, `start_container`, `remove_container` — lifecycle
- `prune_containers`, `prune_images`, `prune_networks`, `prune_volumes` — cleanup
- `create_image` — pull de imágenes
- `events` — stream de eventos Docker (state_worker)

#### 7. JSON Persistence (sin base de datos)

El estado se persiste en archivos JSON en `data/`. Se cargan al startup y se guardan en cada mutación:

- `data/updates_history.json` — historial de actualizaciones
- `data/alerts.json` — configuración de alertas por contenedor
- `data/schedules.json` — tareas programadas
- `data/settings.json` — configuración dinámica (auto-update, Telegram, Matrix)

Cargados al startup via `load_json::<T>()` y guardados via `json_writer()` (flush + rename atómico).

#### 8. Configuration

Cargada desde `config.yaml` (YAML) con override de variables de entorno. Soporta **Podman Secrets** (`/run/secrets/<name>`):

- `HOST`, `PORT`, `SCAN_INTERVAL_SECS`, `ALLOWED_CONTAINERS`
- `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`, `OIDC_REDIRECT_URL`
- `ALERTS` (inline en YAML), `SCHEDULE` (inline en YAML)

#### 10. Frontend Components

| Component | Líneas | Propósito |
|---|---|---|
| `App.tsx` | 300 | Shell principal, tabs, SSE conexiones, layout header |
| `DashboardPage.tsx` | 1474 | Lista de contenedores, batch check/update, inspect, políticas |
| `ConfigPage.tsx` | 686 | Config de notificaciones, auto-update, tema, export/import |
| `LoginScreen.tsx` | 31 | Pantalla de login OIDC |
| `PolicyActionButton.tsx` | 142 | Modal de configuración de política por contenedor |
| `ErrorBoundary.tsx` | — | Error boundary global |
| `NotifToast.tsx` | — | Toast de notificaciones SSE |

#### 11. Mobile Responsive Patterns

El frontend usa `useMediaQuery("(max-width: 768px)")` para detectar mobile. Patrones clave:

- **Header**: 4 botones (Dashboard, Historial, Config, Salir) en una fila, solo emoji, `size="sm"`, gap 4px
- **Container row**: nombre truncado (12→9+`...`), status truncado (20→17+`...`), flecha expand oculta
- **Traefik link**: `Button` solo con `🔗` (no `Anchor` con texto)
- **Policy section**: `Stack` vertical (Política + botón Configurar en dos filas)
- **Check/Desmon buttons**: solo icono (texto en `Tooltip`)
- **Tema**: switch en ConfigPage, no en header
- **Login**: imagen `icon-512x512.jpg`

### API Routes

```
# Auth (OIDC)
GET  /api/auth/login              → OIDC redirect
GET  /api/auth/callback           → OIDC callback (code→token exchange)
GET  /api/auth/me                 → Session info (sub, name, email)
GET  /api/auth/logout             → Clear session cookie

# Containers
GET  /api/containers              → List containers
GET  /api/containers/{name}/inspect → Detailed inspect

# Container lifecycle
POST /api/containers/{name}/start
POST /api/containers/{name}/stop
POST /api/containers/{name}/restart
POST /api/containers/{name}/remove

# Real-time (SSE)
GET  /api/events                  → Container state stream (Docker Events API)
GET  /api/updates                 → Update progress stream
GET  /api/notifications           → Notification stream

# Updates
POST /api/update/{name}           → Pull + restart single container
POST /api/update-all              → Pull + restart all containers
POST /api/check-update/{name}     → Compare local vs Docker Hub

# Stacks (docker-compose)
GET  /api/stacks                  → List compose projects
POST /api/stacks/{project}/update → Pull + recreate stack services

# Admin
GET  /api/admin/alerts            → List alerts
POST /api/admin/alerts            → Create alert
DELETE /api/admin/alerts/{id}     → Delete alert
GET  /api/admin/settings          → Get settings
PUT  /api/admin/settings          → Update settings (auto-update, Telegram, Matrix)

# History
GET  /api/history                 → Update history
DELETE /api/history               → Clear history

# Schedule
GET  /api/schedule                → List scheduled tasks
POST /api/schedule                → Create schedule
DELETE /api/schedule/{id}         → Delete schedule

# Config
GET  /api/config                  → Public configuration (sin secrets)
GET  /api/health                  → Health check (Docker ping)

# Frontend (catch-all)
GET  /*                           → SPA fallback (frontend/dist/index.html)
```

### Justfile Commands

```sh
just list       # List available commands
just check      # Pre-commit: cargo fmt --check + cargo clippy -D warnings
just lint       # cargo clippy --all-targets --all-features
just fmt        # cargo fmt -- --check
just fmt-fix    # cargo fmt
just build      # Build Docker image via Dockerfile
just push       # Push to registry
just upgrade    # Bump version, update deps, tag, build & push

just gf-feature <name>     # Crear feature branch desde develop
just gf-finish <name>      # Merge feature a develop con --no-ff
just gf-release <version>  # Crear release branch desde develop
just gf-publish <version>  # Release: merge a main + tag + merge a develop
just gf-hotfix <desc>      # Crear hotfix branch desde main
just gf-hotfix-publish <desc> <version>  # Publicar hotfix
just gf-graph              # Mostrar árbol de ramas (últimos 30 commits)
```

### Development Workflow

1. Editar backend (`backend/src/*.rs`) o frontend (`frontend/src/`)
2. Pre-commit: `cd backend && just check` (fmt + clippy, **obligatorio**)
3. Test backend: `cd backend && cargo test`
4. Build frontend: `cd frontend && npm run build`
5. Test local: `cd backend && cargo run` (necesita `config.yaml` con OIDC configurado)
6. Producción: `just build && just push`

### Key Dependencies

| Crate | Versión | Propósito |
|---|---|---|
| `axum` | 0.8 | HTTP framework (routes, extractors, middleware, SSE) |
| `bollard` | 0.18 | Docker Engine API client |
| `tokio` | 1 | Async runtime (full features) |
| `jsonwebtoken` | 10 | JWT validation (RS256 via JWKS), CryptoProvider explícito |
| `reqwest` | 0.12 | HTTP client (OIDC discovery, token exchange, Docker Hub API) |
| `serde` / `serde_json` / `serde_yaml` | — | Serialización |
| `tower-http` | 0.6 | CORS middleware + auth middleware |
| `chrono` | 0.4 | Timestamps |
| `cron` | 0.15 | Parseo de expresiones cron (5-field) |
| `cookie` | 0.18 | Session cookie creación/parseo |
| `uuid` | 1 | IDs para alerts/schedules |
| `base64` | 0.22.1 | Decodificación base64 URL-safe para JWKS |
| `tokio-util` | 0.7 | IO utilities |
| `async-stream` | 0.3 | Streams asíncronos |
| `futures` | 0.3 | Stream combinators |
| `tracing` / `tracing-subscriber` | — | Logging estructurado (JSON) |

### Common Development Tasks

- **Añadir ruta API**: Crear handler en el módulo correspondiente + `.route()` en `main.rs`
- **Añadir worker**: Crear async fn en `workers.rs` + `tokio::spawn()` en `main()`
- **Añadir opción de config**: Campo en `Config` + lógica de carga + override env var
- **Añadir estado persistente**: `Arc<Mutex<Vec<T>>>` en `AppState` + `load_json`/`json_writer`
- **Añadir evento SSE**: Struct + `broadcast::Sender` en `AppState` + ruta SSE
- **Nuevo módulo**: `mod name;` en `main.rs` + archivo `backend/src/name.rs`

### Notes

- **OIDC es obligatorio** — no existe fallback a JWT simple. El servidor aborta si faltan vars OIDC.
- **jsonwebtoken v10+** requiere `DEFAULT_PROVIDER.install_default()` explícito al startup.
- La cookie de sesión se firma con `oidc_client_secret` (no hay `SESSION_SECRET` separado).
- `JwtValidator` es estilo PocketID/oxinbox: usa JWKS en vez de secret compartido.
- Docker Compose stacks se descubren automáticamente via `docker compose ls --format json`.
- El cron parsing usa la crate `cron` 0.15 (soporta 5-field estándar).
- `data/` se crea automáticamente al startup si no existe.
- Las alertas son **simples**: solo monitorizan cambios de estado (running→exited→running).
- No hay health checks HTTP/PING — se eliminaron en la limpieza masiva.
- No hay terminal web ni logs en tiempo real por SSE — se eliminaron.
- El frontend usa **Ant Design** v6 y cookies httponly para autenticación (no localStorage).
- El tema oscuro/claro se configura desde ConfigPage (no en header), persiste en `localStorage("color-scheme")`.
- Tests: 44 tests backend (auth: 10, config: 12, models: 13, persistence: 4, workers: 5) + 18 tests frontend (Vitest + Testing Library).

### Estado Actual (julio 2026)

- **Versión**: 0.8.0
- **Backend**: 13 módulos, ~4.235 líneas
- **Frontend**: 5 tabs (Dashboard, History, Alerts, Schedule, Config)
- **Auth**: Solo OIDC (PocketID), sin JWT simple
- **Alertas**: Solo estado de contenedor, sin CPU/RAM
- **Tests**: 44 backend + 18 frontend
- **Build**: Docker multi-stage, just + vampus
