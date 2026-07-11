# Alloy — Plan de Trabajo Integral

## Estado Actual
- ✅ Backend refactorizado de main.rs monolítico → 14 módulos (pattern quadly)
- ✅ Compila con `cargo clippy` 0 errores 0 warnings
- ✅ **30/32 issues completados**

---

## 🔴 Prioridad Alta — Seguridad ✅

### S-1. OIDC sin validación de firma ✅
**Solución:** Validación con JWKs descubiertos del provider. `DecodingKey::from_jwk()` en callback.

### S-2. `jwt_secret` solo env var ✅
**Solución:** `jwt_secret` eliminado de config.yaml. Solo `JWT_SECRET` env var.

### S-3. Endpoints públicos peligrosos ✅
**Solución:** Auth middleware protege todas las rutas excepto `/api/auth/*`, `/api/login`, `/api/health`.

### S-4. CSRF states OIDC expiran ✅
**Solución:** Tupla `(String, Instant)` con TTL 10min + worker de limpieza cada 5min.

### S-5. JWT con expiración ✅
**Solución:** `exp` claim 7 días en simple_login y OIDC callback.

---

## 🟡 Prioridad Media — Calidad de Código ✅

### C-1. `find_container_by_name` extraído ✅
**Solución:** Helper en containers.rs, reemplaza ~15 patrones duplicados. Retorna `Result<_, AppError>`.

### C-2. `reqwest::Client` global ✅
**Solución:** `http_client()` static OnceLock en state.rs. Usado en notifications, workers, updates, auth, main.

### C-3. CRON con crate `cron` ✅
**Solución:** Migrado de parser casero a `cron::Schedule::includes()`. Soporta rangos (`*/5`), listas (`1,3,5`).

### C-4. `load_json` loggea errores ✅
**Solución:** `tracing::warn!` en fallos de lectura/parse.

### C-5. `save_json` loggea errores ✅
**Solución:** `tracing::warn!` en fallos de serialización/escritura.

### C-6. AppError enum ✅
**Solución:** `AppError` con `NotFound`, `Docker`, `Internal` + `IntoResponse`. `From<StatusCode>` y `From<bollard::Error>`. Todos los handlers actualizados.

### C-7. Magic strings eliminados ✅
**Solución:** `strip_name()`, `FILE_*` paths, `LABEL_COMPOSE_*` constants. Reemplazados todos los `trim_start_matches('/')`, rutas JSON y labels compose.

### C-8. Terminal exec restricciones ✅
**Solución:** Blocklist de comandos peligrosos (`rm -rf /`, fork bomb, etc) + audit log con `tracing::info!`.

### C-9. Graceful shutdown ✅
**Solución:** `shutdown_signal()` maneja SIGINT/SIGTERM vía `tokio::select!`. `with_graceful_shutdown()` en axum::serve.

### C-10. Rate limiting ✅
**Solución:** Sliding window (100 req/min per IP). Middleware `rate_limit_mw` en auth.rs con `Arc<Mutex<HashMap>>`.

---

## 🟠 Prioridad Media-Baja — Performance (5/5 ✅)

### P-1. Polling vs Docker Events API ✅
**Solución:** Migrado state_worker a `docker.events()` con filtro de eventos CONTAINER (start, stop, die, kill, pause, unpause, restart, create, destroy, rename, update). Fallback de 30s por si se pierde algún evento. Reconexión automática en errores de stream.

### P-2. `list_containers` redundante ✅
**Solución:** `cached_containers: CachedContainers` en AppState. `state_worker` actualiza cache tras cada fetch. `list_containers_h` lee del cache directamente, evitando llamadas redundantes a Docker API.

### P-3. JSON persistence batching ✅
**Solución:** `JsonWriter` con canal `mpsc::unbounded_channel` + flush worker cada 5s. Deduplicación de escrituras al mismo archivo. Flush forzado si buffer llega a 20 ops. Acceso vía `json_writer()` global static. Reemplazadas 13 llamadas a `save_json()`.

### P-4. Terminal sessions cleanup ✅
**Solución:** Tokio task post-SSE que limpia sender del HashMap si receiver_count == 0 tras 10s.

### P-5. Logging de broadcast overflow ✅
**Solución:** Ya implementado vía `tracing::warn!` cuando receiver lento es droppeado.

---

## 🔵 Prioridad Media — Frontend ✅

### F-1. App.tsx dividido ✅
**Solución:** App.tsx 1719→94 líneas. 8 componentes extraídos: LoginScreen, NotifToast, DashboardPage, LogsPage, StatsPage, VolumesPage, NetworksPage, ConfigPage.

### F-2. TypeScript strict mode ✅
**Solución:** `strict: true`, `strictNullChecks: true`, `noImplicitAny: true` en tsconfig.app.json.

### F-3. Error Boundary ✅
**Solución:** Componente ErrorBoundary con fallback UI y botón recargar. Envuelve App en main.tsx.

### F-4. SSE reconnect hook ✅
**Solución:** `useSSE` hook con exponential backoff (hasta 10 retries). Usado en pages con SSE.

### F-5. Loading states ✅
**Solución:** Todos los componentes tienen estado loading con Loader + texto "Conectando...".

### F-6. Tipos compartidos ✅
**Solución:** `types.ts` con interfaces compartidas. Backend y frontend sincronizados manualmente.

### F-7. API URLs centralizadas ✅
**Solución:** `api.ts` con función `apiFetch()` + helpers. Sin URLs hardcodeadas en componentes.

### F-8. Cleanup en useEffect ✅
**Solución:** Todos los SSE/useEffect tienen return cleanup. `useSSE` hook maneja Lifecycle.

---

## ⚪ Prioridad Baja — Testing & Infra (2/4 ✅)

### T-1. Tests ✅
**Solución:** 88 tests (68 backend en 6 módulos + 20 frontend en 4 suites). Backend: `cargo test` + `cargo clippy` 0 warnings. Frontend: `vitest run` con @testing-library/react y jsdom.

### T-2. Health check ✅
**Solución:** `GET /api/health` → `{"status":"ok","docker":true}` con `docker.ping()`. Sin auth.

### T-3. Logging JSON ✅
**Solución:** `tracing_subscriber::fmt().json().with_env_filter(EnvFilter::from_default_env())`.

### T-4. CI/CD ❌
**Pendiente:** GitHub Actions workflow.

---

## Resumen

| Prioridad | Count | Estado |
|-----------|-------|--------|
| 🔴 Seguridad | 5 | ✅ 5/5 |
| 🟡 Código | 10 | ✅ 10/10 |
| 🟠 Performance | 5 | ✅ 5/5 |
| 🔵 Frontend | 8 | ✅ 8/8 |
| ⚪ Testing/Infra | 4 | ✅ 3/4 |
| **Total** | **32** | **✅ 31/32** |

## Legend
- `[ ]` — pendiente
- `[x]` — completado