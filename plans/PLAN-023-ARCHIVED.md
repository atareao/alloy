# PLAN-023 — Paginación en servidor del listado de contenedores (#166, parte 2)

**Issue:** #166
**Change OpenSpec:** `openspec/changes/container-list-pagination/`
**Rama:** `feature/container-list-pagination` (desde `develop`)

## Objetivo

Paginar y filtrar en el servidor el listado de contenedores (REST y tiempo real), con tamaño de página 10/20/50 seleccionable en la UI, sin romper el estado por fila ni las actualizaciones en vivo.

## Contrato

`GET /api/containers?page=1&page_size=20&q=&state=&has_update=` → `{ items, total, page, page_size }`. `state` admite repetición (multi-selección): `?state=running&state=exited` filtra por varios estados a la vez (semántica OR).
`page_size` en {10,20,50} (def. 20); `page` >= 1 (def. 1); inválidos → 400. Orden `(compose_project, name)`.
`/api/state` y `/api/stream` aceptan los mismos parámetros; el `snapshot` entrega la página y `StateResponse` añade `total`/`page`/`page_size`. `summary` global.
Tolerancia: escalares repetidos → gana el último (sin 400); desconocidos → ignorados (valor por defecto); `state` es el único repetible con OR.

## Mapa de cambios

- Backend: `containers.rs` (query + envelope), `models.rs` (`PagedContainers`, `StateResponse`), `events.rs` (`Query` en `stream_h`/`state_h`, paginación al serializar).
- Frontend: `App.tsx` (filtros + query + refetch + debounce), `useRealtime.ts`, `useStatePoll.ts`, `DashboardPage.tsx` (filtros como props, colapso), `ContainerTable.tsx` (`Pagination`), `types.ts`.
- Tests: backend (paginación pura + serialización) y frontend (`App`, `useRealtime`, `test-setup`, `DashboardPage`, `ContainerTable`).

## Riesgos

- **Reconexión del SSE** al cambiar de página/filtros: coste de reapertura; el snapshot inicial llega de inmediato.
- **Cambio incompatible** en la forma de `GET /api/containers` (array → envelope): hay que actualizar todos los mocks.
- Los tests de filtrado de `DashboardPage` cambian de naturaleza (pasan a ser de `App`).

## Tareas

1. Backend RED: tests de la función pura + validación de query + serialización.
2. Backend GREEN: query, envelope, `StateResponse`, `stream_h`/`state_h`.
3. Frontend RED: mocks, snapshot, query en las peticiones, `Pagination`.
4. Frontend GREEN: filtros en `App`, debounce, reconexión, paginador, colapso, check-all.
5. Verificación completa (cargo test, just check, pnpm test/tsc/lint/build).
6. Revisión, commit y PR.

## Criterios de aceptación

- [x] `GET /api/containers` responde `{items,total,page,page_size}` con `page_size` en {10,20,50} y rechaza inválidos con 400.
- [x] Filtros `q`/`state`/`has_update` en servidor; `total` filtrado.
- [x] Orden determinista `(compose_project, name)`.
- [x] `/api/state` y `/api/stream` paginan y filtran; `summary` global.
- [x] UI: `Pagination` 10/20/50, filtros al servidor, búsqueda debounced, reconexión del stream.
- [x] Expansión reiniciada al cambiar de página; progreso de check-all con el total global.
- [x] `cargo test`, `just check`, `pnpm test`, `tsc -b`, lint y build en 0.
- [x] Diff limitado a `backend/` y `frontend/` + artefactos OpenSpec/plan.
