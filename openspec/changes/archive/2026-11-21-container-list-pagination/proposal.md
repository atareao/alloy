# Proposal

## Why

Parte 2 del issue #166. Hoy `GET /api/containers` devuelve la lista completa sin paginar y el filtrado (búsqueda, estado y pendientes de actualización) se hace en el cliente sobre esa lista (`DashboardPage.tsx`), mientras el `snapshot` del SSE reenvía la flota entera en cada refresco. Con muchas réplicas crecen sin límite el payload, el render y el coste por refresco. La decisión abierta del issue se resolvió a favor de **paginación en servidor (variante A)**: paginar y filtrar en el servidor, tanto el REST como el tiempo real.

## What Changes

- `GET /api/containers` acepta `page`, `page_size`, `q`, `state` y `has_update`, y responde `{ items, total, page, page_size }`.
- `page` >= 1 (por defecto 1); `page_size` en {10, 20, 50} (por defecto 20). Valores fuera de rango o no numéricos → `400`.
- Tolerancia de la query: parámetros escalares repetidos → gana el último valor (no `400`); parámetros desconocidos → ignorados (el campo cae a su valor por defecto); `state` es el único repetible con semántica OR.
- Filtros: `q` (substring case-insensitive sobre `name`, `image` y `compose_project`), `state` (puede repetirse para filtrar por varios estados a la vez, semántica OR) y `has_update` (`true`/`false`). `total` es el número de contenedores tras aplicar los filtros.
- Orden determinista: `(compose_project, name)` ascendente, para que la paginación sea estable.
- `/api/state` y `/api/stream` aceptan los mismos parámetros. El `snapshot` del SSE entrega solo la página solicitada; `StateResponse` incorpora `total`, `page` y `page_size`.
- El `summary` de `StateResponse` sigue calculándose sobre **toda** la flota (no sobre la página).
- Frontend: los filtros (`searchQuery`, `stateFilter`, `showPendingUpdates`) suben de `DashboardPage` a `App`, que es quien construye la query. `App` refetcha `/api/containers` y reconecta el `EventSource` de `/api/stream` al cambiar página, tamaño o filtros (con debounce en la búsqueda). `ContainerTable` añade el `Pagination` de Ant Design con selector 10/20/50.
- El total del progreso de `check-all` pasa a usar el total global de la flota en lugar de `containers.length`.

## Capabilities

### New Capabilities

- `backend/container-pagination`: contrato paginado y filtrado del listado de contenedores (REST y tiempo real).

### Modified Capabilities

- `frontend/container-list`: el listado pasa a paginarse y filtrarse en el servidor, con controles de paginación en la UI.

## Impact

- Backend: `backend/src/containers.rs`, `backend/src/events.rs`, `backend/src/models.rs`. `backend/src/workers/state.rs` **no** cambia (el broadcast sigue llevando la lista completa; la paginación se aplica al serializar).
- Frontend: `frontend/src/App.tsx`, `frontend/src/useRealtime.ts`, `frontend/src/useStatePoll.ts`, `frontend/src/components/DashboardPage.tsx`, `frontend/src/components/ContainerTable.tsx`, `frontend/src/types.ts`.
- Tests: `App.test.tsx`, `useRealtime.test.tsx`, `test-setup.test.tsx`, `DashboardPage.test.tsx`, `ContainerTable.test.tsx`, y los unitarios de paginación en backend.
- Contrato: cambio **incompatible** en la forma de `GET /api/containers` (array → envelope). Los tests de auth que pegan al endpoint usan un mock y no se ven afectados.
- Issue: #166 (parte 2 de 2).
