# PLAN-015 — backend-hardening (parte 1: puntos 1–3 de #159)

- **Issue:** #159 (🛡️ hardening backend) — este plan cubre los puntos **1–3**; el punto 4 (token por query/SSE) irá en un segundo change/PR.
- **Change OpenSpec:** `backend-hardening` (proposal + specs `backend/error-handling`, `security/headers`, delta `auth/session` + design + tasks)
- **Rama:** `feature/backend-hardening` (desde `develop`)
- **Estado:** planificado

## Objetivo

Tres refuerzos no-breaking del backend:
1. Eliminar `unwrap()` de producción sobre la pool/`Mutex` de conexión → `AppError` (HTTP 500) sin abortar el proceso.
2. Marcar `Secure` la cookie de sesión (opt-out `ALLOW_INSECURE_COOKIES=1` para dev en HTTP).
3. Añadir cabeceras de seguridad (CSP, X-Content-Type-Options, X-Frame-Options, Referrer-Policy, HSTS) a todas las respuestas.

## Alcance

- `backend/src/models.rs` (helper `AppError::from_pool`).
- Handlers con `unwrap` de pool/lock: `admin.rs`, `config.rs`, `updates/handlers.rs` y resto (~55 sitios).
- `backend/src/config.rs` (`allow_insecure_cookies()` desde env).
- `backend/src/auth.rs` (cookie `Secure`; extraer `build_session_cookie` testeable).
- `backend/src/main.rs` (middleware de cabeceras; HSTS si HTTPS / `x-forwarded-proto: https`).

Decisiones: helper central para el mapeo a error; `Secure` por defecto con opt-out por env; CSP compatible con Ant Design (`style-src 'unsafe-inline'`, `script-src 'self'`).

## Tareas (TDD)

1. **RED**: test pool cerrada → handler responde 500 sin panic; test unitario de `build_session_cookie(secure)`; test del router (oneshot) verificando cabeceras + HSTS condicional.
2. **GREEN**: helper `AppError::from_pool`; firmas `Result<Json<T>, AppError>` + `map_err(...)?`; `Config::allow_insecure_cookies`; cookie `Secure`; middleware de cabeceras.
3. **REFACTOR**: `cd backend && just check` (fmt + clippy -D warnings); `cargo test` completo en verde; verificación manual de que la SPA carga con la CSP.
4. **Cierre**: auditar con @rust-reviewer; `docs/backend-hardening.md` (+ nota README sobre Secure/ALLOW_INSECURE_COOKIES); PR a `develop`; CI verde; `openspec archive backend-hardening` + limpieza; actualizar #159 (queda el punto 4 para su propio PR); renombrar a `plans/PLAN-015-ARCHIVED.md` y actualizar `HANDOFF.md`.

## Verificación

- Backend: `cargo test` en verde (baseline 464 OK / 12 ignored, aumentará con los tests nuevos); `just check` limpio.
- La SPA sigue cargando con la CSP aplicada (sin bloqueos).
- Sin cambios de contrato de rutas/JSON; solo cambian códigos de estado ante fallo de BD y se añaden cabeceras.
