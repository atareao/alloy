# PLAN-019 — Tests frontend de Dashboard y Container* (#162)

**Issue:** #162
**Change OpenSpec:** `openspec/changes/frontend-test-coverage/`
**Rama:** `feature/frontend-test-coverage` (desde `develop`)

## Objetivo

Añadir tests colocados para `DashboardPage`, `ContainerRow`, `ContainerTable`, `ContainerActions` e `InspectModal`, mover el test de `ErrorBoundary` a `components/` y mantener la suite verde.

## Mapa de tests

- `frontend/src/components/ErrorBoundary.test.tsx` (movido)
- `frontend/src/components/ContainerActions.test.tsx`
- `frontend/src/components/ContainerRow.test.tsx`
- `frontend/src/components/ContainerTable.test.tsx`
- `frontend/src/components/InspectModal.test.tsx`
- `frontend/src/components/DashboardPage.test.tsx`

> Nota: `ContainerRow` no renderiza la imagen; se documenta en el spec y se cubre en `InspectModal`. Mostrarla en la fila requeriría un change incremental.

## Tareas

1. Mover el test de ErrorBoundary y ajustar el import.
2. Escribir tests de `ContainerActions` (callbacks, running/exited, batch, stack).
3. Escribir tests de `ContainerRow` (renderizado, expansión, acciones, badge).
4. Escribir tests de `ContainerTable` (búsqueda, filtros, check all, grupos, batch).
5. Escribir tests de `InspectModal` (loading, error, vacío, datos, cierre).
6. Escribir tests de `DashboardPage` (filtros, acciones API, inspección, borrado).
7. Ejecutar `npx vitest run`, `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build`.

## Criterios de aceptación

- [x] No existe `frontend/src/ErrorBoundary.test.tsx`.
- [x] Existe `frontend/src/components/ErrorBoundary.test.tsx`.
- [x] Existen tests colocados para los cinco componentes.
- [x] Las acciones de contenedor están cubiertas (start/stop/restart/remove/inspect/logs).
- [x] La suite de frontend pasa y `tsc -b`, lint y build siguen limpios.
- [x] Sin cambios de producción no justificados.
