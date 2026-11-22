# Paginación y filtrado de contenedores en servidor

Change: `container-list-pagination` · Issue #166 (parte 2) · Plan `plans/PLAN-023.md`

## Qué es

Parte 2 del issue #166. El listado de contenedores del dashboard pasa a **paginar y filtrar en el servidor** (decisión "variante A": REST + tiempo real), con tamaño de página **10/20/50** seleccionable. Antes `GET /api/containers` devolvía la flota completa y el filtrado era de cliente, mientras el `snapshot` del SSE reenviaba todo en cada refresco.

## Ficheros implicados

- `backend/src/models.rs` — query (`ContainerQuery`), validación, paginación pura (`build_state_response`) y allow-list `ALLOWED_PAGE_SIZES`.
- `backend/src/containers.rs` — `list_containers_h`.
- `backend/src/events.rs` — `state_h` (long-poll) y `stream_h` (SSE).
- `frontend/src/containerQuery.ts` — nuevo: construcción de la query y constantes de página.
- `frontend/src/App.tsx` — fuente única de la query (página, tamaño, filtros).
- `frontend/src/useRealtime.ts` — reconexión del `EventSource` con la query.
- `frontend/src/useStatePoll.ts` — long-poll de respaldo con la query.
- `frontend/src/components/DashboardPage.tsx` — filtros y colapso por página/filtros.
- `frontend/src/components/ContainerTable.tsx` — monta el `Pagination`.
- `frontend/src/types.ts` — `StateResponse` con `total`/`page`/`page_size`.

## Cómo funciona

- **Contrato:** `GET /api/containers?page=&page_size=&q=&state=&has_update=` → `{items,total,page,page_size}`. `page` ≥ 1 (def. 1); `page_size` ∈ {10,20,50} (def. 20); valores inválidos → **400**; página fuera de rango → `items` vacío con el `total` real.
- **Filtros:** `q` (substring case-insensitive sobre `name`/`image`/`compose_project`), `state` (parámetro **repetible**, semántica OR, comparación exacta y case-sensitive), `has_update` (bool). `total` = recuento filtrado, antes de paginar.
- **Orden determinista** `(compose_project, name)` ascendente (proyecto ausente → `""`), para que la paginación sea estable.
- **Tolerancia de la query:** parámetros escalares repetidos → gana el último; parámetros desconocidos → se ignoran y el campo cae a su default (tolerancia hacia adelante). El `Deserialize` de la query es manual (con `visit_map`) porque `serde_urlencoded` no mapea claves repetidas a un `Vec`.
- **Tiempo real:** `/api/state` (long-poll) y `/api/stream` (SSE) aceptan los mismos parámetros; el `snapshot` entrega SOLO la página y `StateResponse` añade `total`/`page`/`page_size`. El `summary` sigue siendo **GLOBAL** (toda la flota), para no falsear la barra de resumen.
- **Broadcast interno:** `workers/state.rs` sigue llevando la flota completa; la paginación se aplica al serializar cada respuesta. `workers/state.rs` **no** se modificó.
- **Frontend:** los filtros viven en `App` (fuente única de la query), que construye la query con `containerQuery.ts`, refetcha `/api/containers` y reconecta el `EventSource` (y el long-poll de respaldo) al cambiar página/tamaño/filtros. La búsqueda tiene debounce de 300 ms. `page`/`pageSize` son propiedad del cliente (no se adoptan los del servidor). `ContainerTable` monta el `Pagination` (oculto en modo batch) y `DashboardPage` colapsa filas/stacks cuando cambia la página o los filtros debounced. El progreso de `check-all` usa `summary.total` (global).

## Variables/config

Ninguna. Las opciones de tamaño de página están en `frontend/src/containerQuery.ts` (`PAGE_SIZE_OPTIONS`, `DEFAULT_PAGE_SIZE`) y la allow-list del backend en `backend/src/models.rs` (`ALLOWED_PAGE_SIZES`); **deben coincidir**.

## Cómo verificar

```bash
cd backend && cargo test        # 530+ tests
cd backend && just check
cd frontend && pnpm run test    # 192 tests
cd frontend && pnpm exec tsc -b
cd frontend && pnpm run lint
cd frontend && pnpm run build
```

## Decisiones

- **Paginación en servidor (variante A)** en lugar de cliente: el SSE reenvía la flota completa en cada refresco, así que paginar solo el REST habría sido cosmético.
- `summary` **global** aunque haya filtros o página.
- **Reconectar el `EventSource`** al cambiar la query (coste de reapertura, mitigado porque el snapshot inicial llega de inmediato).
- `page`/`pageSize` con **fuente única de verdad en el cliente**, para evitar bucles de refetch.
- `state` **case-sensitive** (coherente con los estados de Docker) mientras `q` es case-insensitive.

## Cambio de comportamiento a tener en cuenta

La agrupación por stack y el contador de "Sin stack" son **por página** (un `compose_project` con más contenedores que `page_size` se reparte entre páginas), y el orden intra-grupo pasa a ser `name` ascendente.

## Follow-ups

- Mostrar la imagen del contenedor en la fila (parte 1 la dejó fuera; change incremental).
- Warnings de `oxlint` (#179).
