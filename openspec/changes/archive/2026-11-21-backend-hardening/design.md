# Design — backend-hardening

## 1. Errores de pool/lock → AppError

**Problema**: ~55 sitios hacen `db_pool.get().await.unwrap()` / `conn.lock().unwrap()` en código
de producción; un fallo de pool o un mutex envenenado aborta el proceso.

**Decisión**: helper central en `AppError` (ya existe en `models.rs`, con `IntoResponse` → JSON
`{"error": ...}`):

```rust
impl AppError {
    pub fn from_pool<E: std::fmt::Display>(e: E) -> Self {
        AppError::Internal(format!("base de datos no disponible: {e}"))
    }
}
```

Los handlers cambian su firma de `-> Json<T>` a `-> Result<Json<T>, AppError>` y sustituyen los
`unwrap()` por `?` (los errores de `deadpool` no implementan `From` hacia `AppError` directamente,
así que se mapea con `map_err(AppError::from_pool)?`). axum soporta handlers que devuelven `Result`.

**Alternativa descartada**: `impl From<deadpool::managed::PoolError<...>> for AppError`. Es viable,
pero introduce un `From` en `models.rs` acoplado al tipo concreto de pool; el helper explícito deja
clara la intención y evita conflictos de coherencia. Se centraliza igualmente.

**Nota**: quedan `unwrap()` en `main.rs` sobre *parsing* de dirección/socket en el arranque (fallo
irrecuperable en startup): se revisan aparte si procede; el alcance de este cambio son los handlers.

## 2. Cookie de sesión `Secure`

`auth.rs` construye la cookie con `.http_only(true).same_site(Lax)`. Se añade `.secure(...)`.

**Decisión**: `Secure` por defecto; opt-out explícito para desarrollo por HTTP vía
`ALLOW_INSECURE_COOKIES=1`. Se añade a `Config` un método `allow_insecure_cookies()` que lee la
variable de entorno (mismo patrón que `Config::load`).

La construcción se extrae a una función pura y testeable
(`fn build_session_cookie(token, max_age_secs, secure: bool) -> Cookie<'static>`) para poder
verificar las flags sin arrancar el servidor.

**Riesgo**: despliegues servidos por HTTP plano romperán el login → documentar en `docs/` y README;
`co1` va detrás de HTTPS, sin impacto.

## 3. Cabeceras de seguridad

**Decisión**: middleware aplicado al router completo (capa más externa, para cubrir también
estáticos y SSE) que fija cabeceras en la respuesta. Se implementa con `axum::middleware::from_fn`
(ya se usa ese patrón en `main.rs`) o `tower_http::set_header::SetResponseHeaderLayer`.

Cabeceras:

| Cabecera | Valor |
|---|---|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'` |
| `Strict-Transport-Security` | `max-age=31536000; includeSubDomains` — solo si la petición llega por HTTPS |

**CSP y Ant Design**: AntD inyecta estilos en runtime → requiere `style-src 'unsafe-inline'`. Los
scripts de la SPA van en ficheros (`script-src 'self'`, sin `unsafe-inline`). `data:` en
`img-src`/`font-src` por iconos/fuentes empaquetados.

**HSTS**: axum no conoce el esquema del cliente detrás de un proxy; se emite cuando
`x-forwarded-proto: https` está presente (o `req.uri().scheme() == https`). Es seguro omitirlo en
HTTP plano (los navegadores lo ignoran igualmente).

## Verificación

- `backend/error-handling`: test con pool **cerrada** que invoca un handler y afirma HTTP 500 sin panic.
- `auth/session`: test unitario de `build_session_cookie` (con `secure=true/false`).
- `security/headers`: test con `tower::ServiceExt::oneshot` sobre el router verificando las cabeceras.
- `cargo test`, `just check` limpios.
