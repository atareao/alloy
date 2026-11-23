# PLAN-034 — ui-pagination-and-vertical-tabs

- **Issue**: #220
- **Change SDD**: openspec/changes/ui-pagination-and-vertical-tabs/
- **Rama**: feature/ui-pagination-and-vertical-tabs (desde develop)
- **Estado**: planificación (SDD)

## Objetivo

Ajustes de interfaz:
1. Dashboard sin paginación (mostrar todo).
2. Historial con paginación por defecto de 10.
3. Config → Notificaciones con pestañas verticales (Telegram, Matrix, Valet).
4. Config → Actualizaciones con pestañas verticales (Revisión, Política).

## Alcance

- Frontend React + TypeScript + Ant Design v6.
- Sin cambios de backend/API.

## Ficheros implicados

- `frontend/src/components/ContainerTable.tsx`
- `frontend/src/components/DashboardPage.tsx`
- `frontend/src/App.tsx`
- `frontend/src/containerQuery.ts`
- `frontend/src/HistoryPage.tsx`
- `frontend/src/historyQuery.ts`
- `frontend/src/components/NotificationsTab.tsx`
- `frontend/src/components/UpdatesTab.tsx`

## Tareas

1. RED: tests de componente para los 4 comportamientos.
2. GREEN: Dashboard sin paginación.
3. GREEN: Historial pageSize 10.
4. GREEN: pestañas verticales en Notificaciones.
5. GREEN: pestañas verticales en Actualizaciones.
6. REFACTOR: lint + tsc + test + build.
7. Docs: `docs/ui-pagination-and-vertical-tabs.md`.

## Criterios de aceptación

- Dashboard renderiza todos los contenedores sin `Pagination`.
- Historial arranca con `page_size=10`.
- Notificaciones y Actualizaciones usan `Tabs tabPosition="left"`.
- `pnpm run test` y `pnpm run build` verdes.
