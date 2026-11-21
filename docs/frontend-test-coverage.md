# Frontend test coverage

Change: `frontend-test-coverage` · Issue #162 · Plan `plans/PLAN-019.md`

## Qué es

Añade tests colocados para los componentes del dashboard (`DashboardPage`, `ContainerRow`, `ContainerTable`, `ContainerActions`, `InspectModal`) y mueve el test de `ErrorBoundary` a `components/`, sin cambios de producción no justificados.

## Ficheros implicados

Tests nuevos:

- `frontend/src/components/ContainerActions.test.tsx`
- `frontend/src/components/ContainerRow.test.tsx`
- `frontend/src/components/ContainerTable.test.tsx`
- `frontend/src/components/InspectModal.test.tsx`
- `frontend/src/components/DashboardPage.test.tsx`

Test movido:

- `frontend/src/ErrorBoundary.test.tsx` → `frontend/src/components/ErrorBoundary.test.tsx` (import `./ErrorBoundary`).

## Cómo funciona

- **ContainerActions**: callbacks `onInspect`, `onStart`, `onStop`, `onRestart`, `onLogs`, `onRemove` con el nombre del contenedor; estado running (Parar) y no-running (Iniciar); modo batch con botones deshabilitados; acciones de stack con varios contenedores del mismo `compose_project`.
- **ContainerRow**: nombre, estado, badge de actualización, expansión y callbacks de acciones.
- **ContainerTable**: búsqueda, filtros de estado, "check all", agrupación por stack y ocultación de la barra en modo batch.
- **InspectModal**: estados loading, error, vacío y datos; pestañas y cierre.
- **DashboardPage**: renderizado de la tabla, filtros por búsqueda/estado/actualizaciones pendientes, invocación de `apiFetch` en acciones (`/api/containers/<name>/restart`, `/remove`), apertura/cierre del modal de inspección y confirmación de borrado.

## Decisiones

- La imagen se muestra en `InspectModal` (`containerInfo.image`), **no** en la fila de `ContainerRow`.
- El borrado no refresca localmente: delega el refresco al SSE.

## Verificación

174 tests en verde con `npx vitest run`; `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` limpios.

## Follow-ups

- Mostrar la imagen en la fila requeriría un change incremental (nuevo change proposal + spec).
