# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 Backend: serialización de `ContainerEvent` (campos name/action/state/timestamp).
- [x] 1.2 Backend: el handler `/api/stream` emite primero un evento `snapshot` con `StateResponse` equivalente a `/api/state`.
- [x] 1.3 Backend: `state_worker::refresh` emite `ContainerEvent` por cada contenedor cuyo estado cambió (y no por los que no).
- [x] 1.4 Backend: `update_progress`/`reset_progress_cache` emiten `update-progress` en el canal.
- [x] 1.5 Backend: se emite `notification` cuando una política con `notify_events` dispara.
- [x] 1.6 Frontend: `useRealtime` procesa los 4 eventos y actualiza estado (test con `EventSourceMock`).
- [x] 1.7 Frontend: `useRealtime` cae a polling cuando `EventSource` falla por encima del umbral.
- [x] 1.8 `cargo test -- --skip test_integration` y `npx vitest run` en rojo; legacy en verde.

## 2. GREEN — Implementación mínima

- [x] 2.1 `models.rs`: `ContainerEvent` (Serialize) + tests.
- [x] 2.2 `state.rs`: `progress_tx`, `notif_tx`, `container_event_tx` en `AppState` y sus `FromRef`.
- [x] 2.3 `main.rs`: crear canales y cablearlos a workers/handlers.
- [x] 2.4 `events.rs`: handler `GET /api/stream` (mux de 4 streams + heartbeat por fusión) y ruta.
- [x] 2.5 `workers/state.rs`: emitir `ContainerEvent` en `refresh` (reusar el diff `prev != curr`).
- [x] 2.6 `updates/handlers.rs`: emitir `UpdateProgress` en `update_progress` y `reset_progress_cache`.
- [x] 2.7 `notifications` / llamadas: emitir `NotifEvent` donde ya hay `notify_all`.
- [x] 2.8 Frontend: `useRealtime` con SSE + fallback y estado de conexión; `types.ts` al día.
- [x] 2.9 `App.tsx`: consumir `useRealtime`; eliminar *workarounds* de fin de batch.
- [x] 2.10 `NotifToast` renderizado desde `notification`; highlight en `DashboardPage` desde `container-event`.
- [x] 2.11 `cargo test` y `npx vitest run` verdes; `cargo check` / `npx tsc -b`.

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 Eliminar `useSSE.ts` (o dejar solo el hook nuevo) y código muerto relacionado.
- [x] 3.2 Actualizar el comentario obsoleto de `main.rs` sobre endpoints SSE.
- [x] 3.3 `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] 3.4 `npx eslint .` / type-check de frontend.
- [x] 3.5 Revisión con `rust-reviewer` y `react-reviewer`.

## 4. Archive

- [x] 4.1 `openspec validate sse-first --strict`
- [x] 4.2 `openspec archive sse-first --yes` + limpieza del directorio activo
