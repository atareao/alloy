# Alloy — Reference (frontend, dependencias, tareas)

> Bloques extraídos **literalmente** de `AGENTS.md`. Las reglas operativas (obligaciones y quirks) permanecen en `AGENTS.md`.

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
