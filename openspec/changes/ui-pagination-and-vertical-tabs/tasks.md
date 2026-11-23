# Tasks — ui-pagination-and-vertical-tabs

## 1. RED
- [x] Tests backend que fallen para `all=true` en `/api/containers`, `/api/state`, `/api/stream`.
- [x] Tests frontend que fallen: Dashboard sin paginación, Historial pageSize 10, pestañas verticales Notificaciones/Actualizaciones.

## 2. GREEN — Backend `all=true`
- [x] Aceptar `all` en `ContainerListQuery` y en la query de tiempo real.
- [x] Devolver la flota completa filtrada con `all=true` (sin validar page/page_size).
- [x] Ajustar `/api/state` y `/api/stream` para reenviar `all`.

## 3. GREEN — Dashboard sin paginación
- [x] Quitar `Pagination` y props de paginación de `ContainerTable.tsx`.
- [x] Quitar props de paginación de `DashboardPage.tsx`.
- [x] Añadir `all=true` a `containerQuery.ts` y quitar el estado de paginación de `App.tsx`.

## 4. GREEN — Historial pageSize 10
- [x] Añadir `DEFAULT_HISTORY_PAGE_SIZE = 10` en `historyQuery.ts`.
- [x] Usarlo como estado inicial en `HistoryPage.tsx`.

## 5. GREEN — Pestañas verticales (Notificaciones)
- [x] Convertir `NotificationsTab.tsx` a `Tabs tabPosition="left"` con items Telegram/Matrix/Valet.

## 6. GREEN — Pestañas verticales (Actualizaciones)
- [x] Convertir `UpdatesTab.tsx` a `Tabs tabPosition="left"` con items Revisión/Política.

## 7. REFACTOR
- [x] `cd backend && just check && cargo test`.
- [x] `cd frontend && pnpm run lint && pnpm exec tsc -b && pnpm run test && pnpm run build`.

## 8. Docs
- [x] Crear `docs/ui-pagination-and-vertical-tabs.md`.
