# Tasks: sse-to-websocket

## Backend (TDD)

- [ ] RED: test `ws_handler_upgrades_on_valid_token` — test de integración que
      conecta vía WebSocket a `/api/ws?token=<válido>` y verifica upgrade 101.
- [ ] RED: test `ws_handler_rejects_without_token` — sin token, responde 401.
- [ ] RED: test `ws_handler_receives_state_events` — conecta WS, envía un
      `StateEvent` al broadcast, verifica que llega como mensaje JSON con
      `type: "containers"`.
- [ ] RED: test `ws_handler_receives_update_progress` — igual con UpdateProgress.
- [ ] RED: test `ws_handler_receives_notifications` — igual con NotifEvent.
- [ ] GREEN: implementar handler WebSocket en `events.rs`:
      - Añadir `features = ["macros", "ws"]` a axum en Cargo.toml
      - Handler `ws_h` con `WebSocketUpgrade` + auth vía `?token=`
      - Bucle `tokio::select!` sobre los 3 broadcast receivers
      - Enviar mensajes JSON con formato `{"type": "...", "data": {...}}`
- [ ] GREEN: registrar ruta `GET /api/ws` en `main.rs`
- [ ] `cargo test` — verde.
- [ ] `cargo check` — sin errores.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` — cero warnings.
- [ ] `cargo fmt`.

## Frontend (TDD)

- [ ] RED: test `useWS_connects_with_token` — mockear fetch para `/api/auth/sse-token`
      y WebSocket, verificar que se llama a ambos en orden.
- [ ] RED: test `useWS_filters_by_event_type` — mock WebSocket envía mensajes de
      distintos tipos, verificar que solo el tipo solicitado llega al callback.
- [ ] RED: test `useWS_retries_on_close` — mock WebSocket se cierra, verificar
      backoff y reconexión.
- [ ] GREEN: crear `frontend/src/useWS.ts` con el hook.
- [ ] GREEN: actualizar `App.tsx` para usar `useWS("/api/ws", ...)` en lugar de
      `new EventSource("/api/stream")`.
- [ ] GREEN: mantener `DashboardPage.tsx` sin cambios (el EventSource de logs
      es otro endpoint no relacionado).
- [ ] `npx vitest run` — verde.
- [ ] `npx tsc -b` — sin errores.
- [ ] `npm run lint` — sin errores.

## Verificación final

- [ ] Confirmar que los endpoints SSE legacy siguen funcionando.
- [ ] `openspec archive sse-to-websocket --yes`