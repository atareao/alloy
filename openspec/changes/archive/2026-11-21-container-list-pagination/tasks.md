# Tasks

## TDD Checklist

### Backend — RED

- [x] Tests unitarios de la función pura de filtrado/orden/paginación (página por defecto, segunda página, página fuera de rango, `q`, `state`, `has_update`, filtros combinados, orden determinista, `total`).
- [x] Tests de rechazo de parámetros inválidos (`page_size=30`, `page=0`).
- [x] Test de deserialización de la query (valores por defecto).

### Backend — GREEN

- [x] Struct de query `Deserialize` con defaults y validación.
- [x] Función pura de filtrado + orden + paginación.
- [x] `list_containers_h`: añadir `Query` y devolver el envelope.
- [x] `models.rs`: tipo de respuesta paginada y campos `total`/`page`/`page_size` en `StateResponse`.
- [x] `events.rs`: `stream_h` y `state_h` con `Query`; aplicar filtrado/paginación al serializar el `snapshot` (el `summary` sigue global).
- [x] Actualizar `test_stream_first_event_is_snapshot_state_response` y `test_state_response_serialization`.

### Frontend — RED

- [x] Actualizar los mocks de `/api/containers` al envelope (`{items,total,page,page_size}`) en `App.test.tsx` y `test-setup.test.tsx`.
- [x] Actualizar los tests de `snapshot` (`App.test.tsx`, `useRealtime.test.tsx`) a `StateResponse` con `total`/`page`/`page_size`.
- [x] Test de que `App` pide `/api/containers` con `q`/`state`/`has_update`/`page`/`page_size` al cambiar filtros y página.
- [x] Test de que el `EventSource` se reabre con la nueva query al cambiar de página.
- [x] Test del `Pagination` (total, cambio de página y de tamaño).
- [x] Reubicar los tests de filtrado de `DashboardPage.test.tsx` (los filtros pasan a ser props y ya no filtran en cliente).

### Frontend — GREEN

- [x] `types.ts`: tipo `PagedContainers` y `StateResponse` con `total`/`page`/`page_size`.
- [x] Subir `searchQuery`, `stateFilter` y `showPendingUpdates` a `App`; pasarlos como props a `DashboardPage`.
- [x] `App`: construir la query, refetch de `/api/containers` y estado `total`/`page`/`pageSize`.
- [x] `App`: debounce de la búsqueda.
- [x] `useRealtime`/`useStatePoll`: aceptar y aplicar la query; reconectar al cambiar.
- [x] `ContainerTable`: `Pagination` con `pageSizeOptions` 10/20/50.
- [x] Colapsar filas/stacks al cambiar de página o filtros.
- [x] `check-all`: total del progreso desde el `summary` global.

### Verificación

- [x] `cd backend && cargo test` → 0.
- [x] `cd backend && just check` → 0.
- [x] `cd frontend && pnpm run test` → 0.
- [x] `cd frontend && pnpm exec tsc -b` → 0.
- [x] `cd frontend && pnpm run lint` → 0.
- [x] `cd frontend && pnpm run build` → 0.
- [ ] Commit con gitmoji (`✨ feat`) y PR a `develop`.
