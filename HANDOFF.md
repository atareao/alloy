# HANDOFF

## Estado: sin tareas pendientes

La feature del histórico paginado (#197) está **completa, mergeada y cerrada**. No hay trabajo en curso.

## Última feature cerrada — paginación y filtros del histórico (#197)

- **Issue**: #197 — cerrado.
- **PRs**: #198 (feature) y #199 (archivado del plan), ambos mergeados en `develop`.
- **Change OpenSpec**: archivado en `openspec/specs/backend/update-history-pagination/` y `openspec/specs/frontend/update-history/`.
- **Plan**: `plans/PLAN-029-ARCHIVED.md`.
- **Doc**: `docs/update-history-pagination.md`.
- **Verificación**: backend 587 tests / frontend 233; CI en verde; auditorías `@rust-reviewer` y `@react-reviewer` aptas.

`GET /api/history` queda paginado y filtrado en el servidor (tamaños 10/20/50, filtros `q` y `status`),
sin cap de 100 y sin el lock del histórico en memoria. `HistoryPage` es dueña de su página y su consulta,
con elipsis real + tooltip, columna única `Versión` y clave de fila estable.

## Follow-ups abiertos (candidatos a su propio change proposal)

1. **Trampa de orden en el workflow SDD**: el hook `pre-commit` exige un proposal activo para commits que
   toquen fuente, y `openspec archive` mueve el proposal a `openspec/changes/archive/`, fuera de su glob.
   Hay que **commitear antes de archivar**. No está documentado en `docs/pre-commit-hook.md`.
2. **`.git/hooks/pre-commit` obsoleto** respecto a la copia canónica `.githooks/pre-commit` (hoy inocuo
   porque `core.hooksPath = .githooks`).
3. **`openspec validate --strict --specs`**: 27 fallos preexistentes por «Purpose section is still a
   placeholder» en `updates/progress` y `updates/update-health-guard`.
4. Deudas de la feature, documentadas en `docs/update-history-pagination.md` § Follow-ups:
   `load_update_history` sigue truncando a 100 filas al arrancar, TOCTOU entre `total` e `items`,
   colisión teórica de `historyRowKey`, columna `Estado` sin cota, `overlayStyle` deprecado en antd v6,
   y la tabla `update_history` sin poda.

## Convenciones que muerden (recordatorio)

- La rama de integración es **`develop`**, no `development`. La rama por defecto es **`main`**, así que
  `Closes #NNN` **no** autocierra al mergear en `develop`: hay que cerrar el issue a mano.
- El historial de `develop` usa **merge commits**, no squashes.
- `git commit -am` deja fuera los ficheros nuevos: usar `git add -A`.
