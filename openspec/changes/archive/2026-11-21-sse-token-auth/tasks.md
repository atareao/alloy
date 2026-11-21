# Tasks

## TDD Checklist

### Backend — tickets SSE

- [x] **RED**: tests de `GET /api/auth/sse-token` (200 con sesión, 401 sin sesión, `purpose = "sse"`, TTL 60 s, `Cache-Control: no-store`, token distinto del JWT de sesión)
- [x] **RED**: tests del middleware con `?token=` (ticket válido en `/api/stream`, JWT de sesión rechazado, ticket fuera de `/api/stream` rechazado, ticket expirado rechazado, ticket en cookie rechazado, cookie de sesión sigue funcionando)
- [x] **RED**: test de que un ticket SSE no valida como `SessionClaims`
- [x] **GREEN**: añadir `SseTicketClaims`, `purpose` opcional en `SessionClaims` y helpers de emisión/validación
- [x] **GREEN**: reescribir `sse_token_h` para emitir el ticket y añadir `Cache-Control: no-store`
- [x] **GREEN**: restringir la extracción por query a `/api/stream` + validación de ticket; insertar `SessionClaims` en extensions para el handler
- [x] **GREEN**: rechazar `purpose` en la validación de sesión
- [x] **REFACTOR**: `cargo clippy -- -D warnings` y `cargo fmt -- --check`
- [x] **VERIFY**: `cargo test` (backend) y `just check`
- [x] **VERIFY**: `cd frontend && npx vitest run` sin regresiones (no se tocó el frontend; vitest no ejecutado por indicación expresa)
- [x] **DOCS**: actualizar `docs/architecture.md` y `docs/backend-hardening.md`; archivar el change
