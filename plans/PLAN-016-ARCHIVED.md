# PLAN-016 — Ticket SSE efímero (#159 punto 4)

**Issue:** #159 (punto 4)
**Change OpenSpec:** `openspec/changes/sse-token-auth/`
**Rama:** `feature/sse-token-auth` (desde `develop`)

## Objetivo

Eliminar el uso del JWT de sesión de larga duración como `?token=` en SSE. Sustituirlo por un ticket firmado, de 60 s y propósito `sse`, aceptado sólo en `GET /api/stream`. Mantener cookie/Bearer para sesión normal y cookie como transporte primario del frontend.

## Mapa de arquitectura

- `backend/src/models.rs`: `SseTicketClaims`, `SseTicketResponse`, `purpose: Option<String>` en `SessionClaims`.
- `backend/src/auth.rs`:
  - `sse_token_h`: emite ticket desde `SessionClaims` + secreto.
  - `auth_middleware`: valida sesión por cookie/Bearer; sólo acepta ticket en query y sólo en `/api/stream`; inserta claims en extensions.
  - `validate_session`: rechaza `purpose`.
- `frontend/src/useRealtime.ts`: sin cambios (cookie + `withCredentials`).
- `docs/architecture.md`, `docs/backend-hardening.md`: documentación.

## Tareas

1. Crear tests RED de emisión del ticket.
2. Crear tests RED del middleware (aceptación/rechazo por ruta y tipo de token).
3. Crear test RED de rechazo de ticket como sesión.
4. Implementar claims, emisión y validación (GREEN).
5. Restringir query token a `/api/stream` (GREEN).
6. Rechazar `purpose` en sesión (GREEN).
7. Refactor + `cargo clippy -- -D warnings` + `cargo fmt -- --check`.
8. Verificar `cargo test`, `just check`, `npx vitest run`.
9. Actualizar docs y archivar el change.

## Criterios de aceptación

- [x] `?token=` con JWT de sesión en `/api/stream` → 401.
- [x] `?token=` con ticket SSE válido en `/api/stream` → 200 SSE.
- [x] Ticket SSE en `/api/containers` → 401.
- [x] Ticket SSE como cookie/Bearer → 401.
- [x] Cookie de sesión en `/api/stream` → 200 SSE.
- [x] `GET /api/auth/sse-token` devuelve ticket de 60 s con `purpose = "sse"` y `Cache-Control: no-store`.
- [x] Backend tests y `just check` limpios; frontend sin regresiones.

## Verificación

- `cd backend && cargo test`
- `cd backend && just check`
- `cd frontend && npx vitest run`
- `cd frontend && pnpm exec tsc -b`
- `cd frontend && pnpm run lint`
