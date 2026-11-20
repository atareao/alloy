# Design — Resumen del check manual con contadores reales

## Contexto (causa raíz)

- `backend/src/updates/handlers.rs::check_and_apply_all` calcula `checked`, `updated` (aplicados), `num_errors` (errores de comprobación), pero **no** `updatable` ni `uptodate`.
- El último evento SSE de progreso antes de limpiar la caché es `reset_progress_cache`, que emite `UpdateProgress { done: true, total: 0, checked: 0, updated: 0, errors: 0 }`. El frontend (`App.tsx::onProgress`) dispara `finishBatch` justo con esos ceros.
- `App.tsx::finishBatch` no escribe `checkResults`/`updateResults` (solo muestra un toast). `SummaryDialog` lee esos estados → siempre 0.

## Decisiones

**D1 — Modelo de contadores.** Ampliar `BatchProgress` (caché) y `UpdateProgress` (SSE) con:
`updatable` (con actualización disponible), `uptodate` (al día) y `failed` (fallos al aplicar).
Se conserva: `checked` (comprobados), `updated` (aplicados con éxito), `errors` (errores de comprobación). Todos con `#[serde(default)]`.

**D2 — Evento de fin con datos reales.** En `check_and_apply_all`, tras el bucle, emitir un evento
`done = true` con los contadores finales reales usando el `checked` **efectivo** (no `total`, para
que la cancelación refleje solo lo procesado). Introducir un helper `emit_batch_done(...)` o un
parámetro `done` en `update_progress`.

**D3 — Limpieza que no pisa.** `reset_progress_cache` sigue limpiando la caché (para el long-poll
`/api/state`) pero emite un evento *idle* con `done = false` (o deja de emitir `done = true`), de modo
que el último `done = true` que vea el frontend sea el de datos reales. Orden garantizado: `emit_batch_done`
→ `reset_progress_cache`.

**D4 — Mapeo en el frontend.** `finishBatch(bp)` rellena:
- `checkResults`: `total = bp.checked`, `updated = bp.updatable`, `uptodate = bp.uptodate`, `failed = bp.errors`, `done = bp.checked`, `errors = []`.
- `updateResults`: `done = bp.updated`, `failed = bp.failed`, `total = bp.checked`, `updated = bp.updated`, `uptodate = 0`, `errors = []`.
`SummaryDialog` ya usa `checkResults.{total,updated,uptodate,failed}` y `updateResults.{done,failed}`; la tarjeta de Actualización se muestra si `updateResults.done + updateResults.failed > 0`.

**D5 — Lógica pura y testeable.** Extraer `mapBatchToResults(bp): { check, update }` (en `types.ts` o un `utils/batch.ts`) para unit-test sin montar la app; `finishBatch` la usa.

**D6 — Cancelación.** El backend, al cancelar, emite su evento final y luego el `done` con el
`checked` efectivo; el frontend muestra lo procesado (no ceros).

## No objetivos

- Cambiar el worker programado ni su resumen (solo comparten el helper; el frontend ignora eventos `done` si `batchPhase !== "active"`).
- Detalle por-contenedor de errores en el resumen (se mantiene en historial/logs; el listado `errors` queda vacío y su tarjeta se oculta).
- Reinterpretar `has_update` en el `Vec<ContainerInfo>` devuelto.

## Riesgos y mitigaciones

- **Orden de eventos**: el `done` real debe llegar antes que el idle de reset → se emiten secuencialmente en el mismo hilo async.
- **Retrocompatibilidad SSE/API**: campos nuevos con `serde(default)`; el frontend antiguo ignora campos extra.
- **Tests sin Docker**: el bucle real necesita daemon; se testean las piezas puras (`mapBatchToResults`, builder del evento `done`, y que `reset` no emite ceros con `done=true`).

## Esbozo de tests

- Backend (`handlers.rs::tests`): el builder/`emit_batch_done` produce `done=true` con los contadores
  dados; `reset_progress_cache` no emite `done=true` con ceros.
- Frontend: `mapBatchToResults` mapea cada campo; `SummaryDialog` renderiza los valores pasados
  (10/3/7/2 y Actualizados/Fallos).
