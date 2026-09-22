# Tasks: ws-auth-fix

## TDD Checklist

- [ ] RED: test `auth_middleware_does_not_modify_websocket_response` — simular
      una petición WebSocket a `/api/ws?token=<válido>`, verificar que la
      respuesta tiene status 101 y que NO tiene header `Set-Cookie`.
- [ ] GREEN: añadir check `if response.status() == StatusCode::SWITCHING_PROTOCOLS`
      en `auth_middleware` después del bloque SSE check.
- [ ] `cargo test` — verde.
- [ ] `cargo check` — sin errores.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` — cero warnings.
- [ ] `cargo fmt`.
- [ ] `openspec archive ws-auth-fix --yes`