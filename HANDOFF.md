# HANDOFF — Paginación y filtros del histórico de actualizaciones (#197)

## Estado: COMPLETADA, pendiente de PR

La feature está implementada, verificada y auditada. Falta abrir el PR a `develop`
(la tarea 5.5 de `tasks.md` queda sin marcar porque el cierre del issue depende del merge).

## Qué se ha hecho

- **Issue**: #197 (asignado a `atareao`).
- **Rama**: `feature/update-history-pagination` (base `develop` = `7cbb538`).
- **Change OpenSpec**: `update-history-pagination` — archivado en `openspec/specs/`
  (`backend/update-history-pagination/` y `frontend/update-history/`).
- **Plan**: `plans/PLAN-029.md`.
- **Doc de la feature**: `docs/update-history-pagination.md`.

`GET /api/history` pasa de devolver una lista recortada a 100 entradas a estar
**paginado y filtrado en el servidor** (tamaños 10/20/50, filtros `q` y `status`
repetible), y `HistoryPage` pasa a ser la dueña de su página y de su consulta.
Incluye elipsis real con tooltip, una única columna `Versión` y clave de fila estable.

## Verificación (ejecutada y confirmada)

- Backend: `cargo test` → **587 passed / 0 failed**; `just check` (fmt + clippy `-D warnings`) limpio.
- Frontend: `pnpm run test` → **233 passed / 0 failed** (19 ficheros); `tsc -b`, `lint` y `build` limpios.
- Estabilidad: suite frontend verde en **5/5 ejecuciones bajo carga de CPU** (12 bucles), 0 avisos `act(...)`.
- Auditorías: `@rust-reviewer` → apto (2 hallazgos cerrados); `@react-reviewer` → apto (2 MEDIO cerrados, 0 bloqueantes).
- E2E HTTP del endpoint: 11 tests nuevos (envelope, `400` de `page=abc`/`page_size=30`/`page=0`, filtros, `DELETE`, y que el `id` **no** se expone).

## Tarea pendiente

1. `git add -A` (ojo: hay ficheros nuevos sin trackear — `docs/update-history-pagination.md`,
   `frontend/src/components/EllipsisText.tsx`, `frontend/src/historyQuery.ts`; un `git commit -am` los dejaría fuera).
2. Commit gitmoji y `git push -u origin feature/update-history-pagination`.
3. Abrir PR a **`develop`** (nunca `main`), referenciando el issue #197.
4. Tras el merge: marcar la tarea 5.5 de `tasks.md` (si el fichero ya no existe porque se archivó,
   basta con cerrar el issue) y renombrar `plans/PLAN-029.md` a `plans/PLAN-029-ARCHIVED.md`.

## Follow-ups conocidos (documentados, no bloqueantes)

Están en `docs/update-history-pagination.md` § «Follow-ups conocidos»: `load_update_history`
sigue truncando a 100 filas al arrancar (solo alimenta el vector en memoria), TOCTOU entre
`total` e `items`, colisión teórica de `historyRowKey`, columna `Estado` sin cota,
`overlayStyle` deprecado en antd v6, y la tabla `update_history` sin poda.
