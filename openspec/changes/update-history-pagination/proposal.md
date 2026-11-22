# Proposal

## Why

`GET /api/history` devuelve hoy un array plano recortado a 100 entradas: el handler ordena por `timestamp` descendente y aplica `sorted.truncate(100)`, y al arrancar la carga se hace con `ORDER BY timestamp DESC LIMIT 100`. No hay paginación ni filtros, y `HistoryPage` pinta todo lo que recibe. La tabla `update_history` **crece sin límite** (no existe poda ni retención) y el vector en memoria también, así que el histórico visible no es el histórico real: todo lo anterior a las últimas 100 filas es inalcanzable. La vista arrastra además dos problemas de presentación: `Container` no trunca, y la versión se parte en dos columnas con corte manual (`shortDigest` → `slice(0,20) + "..."`), justo lo que el propio proyecto prohíbe en `frontend/container-list` («MUST NOT cortar el texto manualmente con `slice`»). Issue #197.

## What Changes

- `GET /api/history` pasa a un contrato paginado `{ items, total, page, page_size }` — **BREAKING** (array → envelope).
- `page` >= 1 (por defecto 1); `page_size` en {10, 20, 50} (por defecto 20); valores fuera de rango o no numéricos → `400`. Se replican las tolerancias ya fijadas para la lista de contenedores: escalares repetidos → gana el último valor; parámetros desconocidos → ignorados; `status` es el único repetible, con semántica OR.
- La página se sirve desde el histórico **persistido** (SQLite) con `LIMIT/OFFSET` y `total` mediante `COUNT(*)` tras filtros, con orden determinista `timestamp DESC, id DESC`. Desaparece el cap de 100 al servir y se añade el índice que soporta ese orden.
- Filtros en servidor: `q` (substring case-insensitive sobre `container`, `image`, `old_digest` y `new_digest`) y `status` (repetible, OR).
- Frontend: `HistoryPage` se pagina con el `Pagination` de Ant Design (10/20/50, en escritorio y en móvil) y pasa a ser la dueña de su estado de página y de su consulta; `App` deja de cargar el histórico y sólo le pasa una señal de refresco que se incrementa al terminar un batch.
- Frontend: `Container` e `Imagen` con elipsis real + tooltip (sin `slice` manual); las columnas «Versión anterior» y «Nueva versión» se funden en una única columna **«Versión»** que muestra la nueva encima de la antigua, ambas con elipsis y tooltip.
- El histórico **no** viaja por SSE: `StateResponse`, `useRealtime`, `/api/stream` y `/api/state` no se tocan.

## Capabilities

### New Capabilities

- `backend/update-history-pagination`: contrato paginado y filtrado del histórico de actualizaciones, servido sobre el histórico persistido.
- `frontend/update-history`: vista del histórico paginada, con elipsis real en las columnas de texto y columna única de versión.

### Modified Capabilities

- Ninguna. Ninguna spec existente cubre `/api/history` ni la vista del histórico, así que no hay requisitos que modificar.

## Impact

- Backend: `backend/src/updates/history.rs` (handler y query), `backend/src/updates/mod.rs` (ruta), `backend/src/db.rs` (consulta paginada, recuento e índice), `backend/src/models.rs` (query y envelope del histórico).
- Frontend: `frontend/src/HistoryPage.tsx` (paginación, columnas y fetch), `frontend/src/App.tsx` (deja de cargar el histórico), `frontend/src/components/EllipsisText.tsx` (nuevo, compartido), `frontend/src/historyQuery.ts` (nuevo, construcción de la query), `frontend/src/types.ts` (tipo del envelope).
- Tests: `backend/src/updates/history.rs` (los 8 unitarios actuales se reescriben), `backend/src/db.rs`, `backend/src/models.rs`; `frontend/src/HistoryPage.test.tsx` (6 casos actuales más los nuevos) y `frontend/src/App.test.tsx`.
- Contrato: cambio **incompatible** en la forma de `GET /api/history` (array → envelope). No hay otros consumidores: el único cliente es `HistoryPage`, y el export de configuración usa el vector en memoria, no este endpoint.
- Datos: migración aditiva (un índice nuevo). La tabla `update_history` no se modifica ni se poda.
- Issue: #197.
