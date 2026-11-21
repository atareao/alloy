# Proposal

## Why

El issue **#159** agrupa cuatro refuerzos de seguridad en el backend. Este cambio cubre los tres
no-breaking (puntos 1–3); el cuarto (token por `?token=`, con implicación en SSE) va en su propio
change proposal.

- **`unwrap` en producción**: los handlers hacen `db_pool.get().await.unwrap()` y
  `conn.lock().unwrap()` (~55 sitios): un fallo de pool o un mutex envenenado **aborta el proceso**
  en lugar de devolver un error HTTP.
- **Cookie de sesión sin `Secure`**: la cookie `session` viaja también por HTTP y puede filtrarse.
- **Sin cabeceras de seguridad**: no hay CSP, `X-Content-Type-Options`, `X-Frame-Options`,
  `Referrer-Policy` ni HSTS.

## What Changes

- **Errores de pool/DB**: sustituir los `unwrap()` de producción por un helper único que mapee
  `PoolError`/`PoisonError` a `AppError::Internal` (HTTP 500, JSON `{"error": ...}`). Sin panics.
- **Cookie de sesión**: añadir `Secure` a la cookie `session` (con opt-out explícito para desarrollo
  local en HTTP vía `ALLOW_INSECURE_COOKIES=1`, por defecto seguro).
- **Cabeceras de seguridad**: middleware que añade a todas las respuestas
  `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`,
  `Content-Security-Policy` compatible con la SPA (AntD) y `Strict-Transport-Security` cuando aplica.

## Capabilities

### New Capabilities

- `backend/error-handling`: los handlers no MUST hacer `unwrap` sobre la pool ni sobre la conexión;
  un fallo MUST traducirse a `AppError` (500) sin abortar el proceso.
- `security/headers`: todas las respuestas MUST incluir las cabeceras de seguridad definidas.

### Modified Capabilities

- `auth/session`: la cookie de sesión MUST marcarse `Secure` (además de `HttpOnly` y `SameSite=Lax`).

## Impact

- **Backend**: `backend/src/admin.rs`, `backend/src/config.rs`, `backend/src/updates/handlers.rs`
  (y otros con `unwrap` de pool), `backend/src/models.rs` (helper `AppError`),
  `backend/src/auth.rs` (cookie), `backend/src/main.rs` (middleware de cabeceras), `backend/src/config.rs`
  (`ALLOW_INSECURE_COOKIES`).
- **API**: sin cambios de contrato en rutas/JSON; cambia el **código de estado** ante fallo de BD
  (500 en vez de panic) y se añaden cabeceras.
- **Ops**: `Secure` puede romper el login en despliegues servidos por HTTP plano (deben usar HTTPS o
  activar el opt-out). Documentar en `docs/` y en el README.
- **Dependencias**: ninguna nueva (CSP/cabeceras se añaden con `axum`/`tower`).
