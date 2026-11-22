# Tasks — backend-hardening

## 1. RED (tests que fallan)

- [x] 1.1 Error handling: test con pool cerrada → handler devuelve HTTP 500 con JSON `{"error": ...}` y no panickea (`tests::closed_pool_handler_returns_500_json_without_panicking`).
- [x] 1.2 Cookie: test unitario de `build_session_cookie` — `secure=true` incluye `Secure`; `secure=false` no.
- [x] 1.3 Headers: test del router (`oneshot`) que verifica cabeceras; HSTS con `x-forwarded-proto: https`.
- [x] 1.4 `cargo test`: 1.1–1.3 fallan (RED: 465 passed / 4 failed) y el resto verde.

## 2. GREEN (implementación mínima)

- [x] 2.1 `models.rs`: helper `AppError::from_pool`.
- [x] 2.2 Handlers: firmas `Result<Json<T>, AppError>` y `map_err(AppError::from_pool)?` (`admin.rs`, `config.rs`, `updates/*`, `containers.rs`).
- [x] 2.3 `config.rs`: `Config::allow_insecure_cookies()` (env `ALLOW_INSECURE_COOKIES`).
- [x] 2.4 `auth.rs`: `build_session_cookie(token, max_age, secure)` + `Secure` en callback y sliding refresh.
- [x] 2.5 Middleware de cabeceras en `main.rs::build_app` (capa externa); HSTS condicionado a HTTPS.
- [x] 2.6 `cargo test` en verde (469 passed / 12 ignored / 0 failed).

## 3. REFACTOR y verificación

- [x] 3.1 `cd backend && just check` (fmt + clippy `-D warnings`).
- [x] 3.2 `cd backend && cargo test` completo en verde.
- [ ] 3.3 Verificación manual: la SPA carga con la CSP aplicada (sin bloqueos en consola) — pendiente de comprobar en co1 tras desplegar.

## 4. Correcciones de auditoría (@rust-reviewer)

- [x] 4.1 ALTO: `fetch_containers` propaga el error (`Result`); `list_containers_h` → 500 ante fallo de BD (no `200 []`).
- [x] 4.2 MEDIO: todos los best-effort registran el fallo (`tracing::warn!`).
- [x] 4.3 MEDIO: `update_container_h`/`update_all_h` (rama pull-fallido) tratan el historial como best-effort (no `?`; no se pierde `results`).
- [x] 4.4 BAJO: test de cabeceras sobre el fallback SPA/estático (`security_headers_present_on_static_spa_fallback`).
- [x] 4.5 Re-auditoría final `@rust-reviewer` (veredicto apto).

## 5. Cierre

- [x] 5.1 `docs/backend-hardening.md` (+ nota README sobre `Secure`/`ALLOW_INSECURE_COOKIES`).
- [x] 5.2 PR de `feature/backend-hardening` a `develop`, CI verde.
- [x] 5.3 `openspec archive backend-hardening` + limpieza.
- [x] 5.4 Actualizar #159 (queda el punto 4 → change/PR propio) y renombrar el plan a `plans/PLAN-015-ARCHIVED.md`; actualizar `HANDOFF.md`.
