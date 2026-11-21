# Hardening del backend (#159, puntos 1–3)

Cambio OpenSpec: `backend-hardening` (capabilities `backend/error-handling`, `security/headers`,
delta `auth/session`). Plan: `plans/PLAN-015-ARCHIVED.md`. Issue: #159.

## Qué es

Tres refuerzos no-breaking del backend:

1. **Errores de pool/lock → `AppError`**: se eliminan los `unwrap()` de producción sobre
   `db_pool.get().await` y `conn.lock()`; un fallo se traduce a `AppError` (HTTP 500) en lugar de
   abortar el proceso.
2. **Cookie de sesión `Secure`**: la cookie `session` se marca `Secure` por defecto.
3. **Cabeceras de seguridad**: middleware que añade CSP y cabeceras anti-clickjacking/sniffing a
   todas las respuestas.

## Ficheros implicados

- `backend/src/models.rs` — helper `AppError::from_pool`.
- Handlers con pool/lock: `backend/src/config.rs`, `backend/src/admin.rs`,
  `backend/src/updates/handlers.rs`, `backend/src/updates/history.rs`, `backend/src/updates/common.rs`,
  `backend/src/containers.rs`.
- `backend/src/config.rs` — `Config::allow_insecure_cookies()` (env `ALLOW_INSECURE_COOKIES`).
- `backend/src/auth.rs` — `build_session_cookie(token, max_age_secs, secure)` + sliding refresh.
- `backend/src/main.rs` — `build_app(state)` (router extraído, testeable) + middleware de cabeceras.

## Cómo funciona

### Errores

Helper central:
```rust
impl AppError {
    pub fn from_pool<E: std::fmt::Display>(e: E) -> Self {
        AppError::Internal(format!("base de datos no disponible: {e}"))
    }
}
```
Los handlers cuya **respuesta depende** de la conexión devuelven `Result<Json<T>, AppError>` y usan
`map_err(AppError::from_pool)?`. Los usos **best-effort** (escrituras laterales de un batch, p. ej.
timestamps) usan `if let Ok(...)` + `tracing::warn!`, sin panickear. Restricción importante: un
`MutexGuard` de `conn.lock()` (no-`Send`) **nunca** debe cruzar un `.await` (rompería el `Handler`
de axum); las secciones que tocan la conexión son síncronas.

### Cookie

`build_session_cookie(token, max_age_secs, secure)` fija `HttpOnly`, `SameSite=Lax` y, si
`secure`, `Secure`. `auth_callback` y el sliding refresh pasan `!config.allow_insecure_cookies()`.

### Cabeceras

Middleware en `build_app` (capa externa, cubre API, estáticos y SSE):

| Cabecera | Valor |
|---|---|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'` |
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains` (solo si HTTPS / `x-forwarded-proto: https`) |

## Variables / config

- **`ALLOW_INSECURE_COOKIES`** (opcional): si es `1/true/yes/on`, la cookie de sesión **no** lleva
  `Secure`. Necesario solo para desarrollo local servido por **HTTP plano**. Por defecto desactivado
  (cookie `Secure`).

## Cómo habilitar / desplegar

- Sin migraciones. Basta con reconstruir la imagen.
- **Importante**: si sirves Alloy por HTTP plano (sin proxy TLS), el login fallará hasta que actives
  `ALLOW_INSECURE_COOKIES=1` o pongas HTTPS por delante. `co1` va detrás de HTTPS → sin cambios.
- La CSP requiere `style-src 'unsafe-inline'` por Ant Design; no relajes `script-src`.

## Decisiones

- Helper explícito `AppError::from_pool` en vez de `impl From<PoolError>` (evita acoplar `models.rs`
  al tipo concreto de pool).
- Distinción deliberada respuesta-dependiente (500) vs best-effort (log + continuar): un fallo al
  persistir timestamps al final de un batch no debe tumbar la respuesta de `check-all`.
- HSTS condicionado a HTTPS (omitirlo en HTTP plano es seguro: los navegadores lo ignoran).

## Follow-ups

- **Punto 4 de #159** (token por `?token=`/SSE) → change/PR aparte.
- Los `unwrap()` de arranque en `main.rs` (parsing de socket/addr) son fallo irrecuperable de
  startup; se dejan como están.
