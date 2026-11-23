# ui-pagination-and-vertical-tabs

## Why

Ajustes de interfaz solicitados por el usuario: el Dashboard debe mostrar la totalidad de los contenedores (sin paginación), el Historial debe arrancar con una página más pequeña (10) y la Configuración debe organizar Notificaciones y Actualizaciones en pestañas verticales. Como `GET /api/containers` está paginado en el servidor, "mostrar todo" exige que el backend pueda devolver la flota completa (modo `all=true`).

## What Changes

- **Backend**: `GET /api/containers`, `/api/state` y `/api/stream` aceptan `all=true`, que devuelve todos los contenedores filtrados sin paginar. Con `all` ausente, la paginación actual no cambia.
- **Dashboard** (`frontend/container-list`): se eliminan los controles de paginación (`Pagination`) y su estado; el cliente solicita `all=true` y renderiza la lista completa.
- **Historial** (`frontend/update-history`): el tamaño de página por defecto pasa de 20 a 10.
- **Config** (`ui/config-page`): Notificaciones (Telegram/Matrix/Valet) y Actualizaciones (Revisión/Política) pasan a pestañas verticales (`Tabs tabPosition="left"`).

## Impact

- Backend: `backend/src/containers.rs`, `backend/src/state.rs`/handlers de `/api/state` y `/api/stream`, `backend/src/events.rs` si aplica.
- Frontend: `ContainerTable.tsx`, `DashboardPage.tsx`, `App.tsx`, `containerQuery.ts`, `HistoryPage.tsx`, `historyQuery.ts`, `NotificationsTab.tsx`, `UpdatesTab.tsx`.
- Specs afectadas: `backend/container-pagination`, `frontend/container-list`, `frontend/update-history`, `ui/config-page`.
- Tests: backend (`cargo test`) y frontend (`pnpm run test`).

## Tasks

- [ ] 1. RED — tests backend y frontend que fallan.
- [ ] 2. GREEN backend — modo `all=true` en `/api/containers`, `/api/state`, `/api/stream`.
- [ ] 3. GREEN frontend — Dashboard sin paginación usando `all=true`.
- [ ] 4. GREEN frontend — Historial pageSize por defecto 10.
- [ ] 5. GREEN frontend — pestañas verticales en Notificaciones.
- [ ] 6. GREEN frontend — pestañas verticales en Actualizaciones.
- [ ] 7. REFACTOR — `just check`, `cargo test`, `pnpm run lint`, `pnpm exec tsc -b`, `pnpm run test`, `pnpm run build`.
- [ ] 8. Docs — `docs/ui-pagination-and-vertical-tabs.md`.
