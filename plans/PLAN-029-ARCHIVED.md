# PLAN-029 — Paginación y presentación del histórico de actualizaciones

- **Issue**: #197 (https://github.com/atareao/alloy/issues/197)
- **Change OpenSpec**: `update-history-pagination`
- **Estado**: COMPLETADA — implementada, verificada y auditada; mergeada en `develop` vía PR #198; change archivado en `openspec/specs/`; issue #197 cerrado.
- **Base gitflow**: rama `feature/update-history-pagination` desde `develop`; PR a `develop`.

## Objetivo

Que la vista del histórico de actualizaciones pagine y filtre en el servidor sobre el histórico **persistido** completo (tamaños 10/20/50), y que su tabla deje de cortar texto a mano: `Container` e `Imagen` con elipsis real y tooltip, y una única columna `Versión` con la nueva encima de la antigua.

## Problema

- `backend/src/updates/history.rs:15` — `sorted.truncate(100)` al servir; `backend/src/db.rs:397` — `ORDER BY timestamp DESC LIMIT 100` al arrancar. El histórico anterior a esas 100 filas es inalcanzable.
- `backend/src/db.rs:455` — `append_update_history` inserta sin cap y no existe poda: la tabla crece sin límite, así que el recorte de 100 esconde datos reales.
- `GET /api/history` no acepta query params y devuelve un array plano (`history.rs:9-17`), mientras la lista de contenedores ya tiene envelope paginado (`openspec/specs/backend/container-pagination/spec.md`).
- `frontend/src/HistoryPage.tsx:204-207` — `shortDigest` corta con `substring(0,20) + "..."`, prohibido por `openspec/specs/frontend/container-list/spec.md` («MUST NOT cortar el texto manualmente con `slice`»).
- `frontend/src/HistoryPage.tsx:210-312` — dos columnas separadas («Versión anterior» y «Nueva versión») y `Container` sin elipsis.
- `frontend/src/HistoryPage.tsx:463-477` — la `Table` se monta con `pagination={false}`.

## Alcance

1. **Paginación en servidor** de `GET /api/history` con envelope `{items,total,page,page_size}`, `page >= 1` (default 1), `page_size` en {10,20,50} (default 20), inválidos → 400, tolerancias de query idénticas a contenedores.
2. **Fuente de datos**: histórico persistido (SQLite) con `LIMIT/OFFSET` y `total` por `COUNT(*)` tras filtros; orden `timestamp DESC, id DESC`; índice `idx_update_history_ts_id`.
3. **Filtros**: `q` (case-insensitive sobre `container`, `image`, `old_digest`, `new_digest`) y `status` (repetible, OR).
4. **Frontend — paginación**: `Pagination` de AntD con 10/20/50 en escritorio y móvil; `HistoryPage` dueña de página, tamaño, filtros y `fetch`; `App` sólo pasa `refreshToken`.
5. **Frontend — presentación**: elipsis real + tooltip en `Container` e `Imagen`; columna única `Versión` con la nueva encima de la antigua, ambas con elipsis; digest vacío → `-`; misma composición en la tarjeta móvil.
6. **Clave de fila estable** (compuesta, sin usar el índice de página).

## Decisiones de diseño

- **D1** La página se sirve desde SQLite, no desde el vector en memoria: el vector sólo tiene las 100 filas previas al arranque más lo de runtime, así que paginar sobre él daría un `total` falso. Efecto colateral: la lectura deja de tomar `update_history.lock()`, con lo que la regla del issue #170 se cumple por construcción.
- **D2** Filtros `q` y `status`, reutilizando el patrón de contenedores.
- **D3** El estado de página vive en `HistoryPage`, no en `App` (no hay stream que necesite la misma query, a diferencia de contenedores).
- **D4** Componente compartido `EllipsisText` usado en `HistoryPage`; `ContainerRow` e `InspectModal` no se migran en esta feature.
- **D5** Orden determinista `timestamp DESC, id DESC` más índice aditivo.
- **D6** Clave de fila compuesta derivada de los datos, sin exponer `id` en la API.
- **D7** `Versión` unifica `old_digest` y `new_digest` y se elimina `shortDigest`.

## Ficheros afectados

- Backend: `backend/src/updates/history.rs`, `backend/src/updates/mod.rs`, `backend/src/db.rs`, `backend/src/models.rs`.
- Frontend: `frontend/src/HistoryPage.tsx`, `frontend/src/App.tsx`, `frontend/src/components/EllipsisText.tsx` (nuevo), `frontend/src/historyQuery.ts` (nuevo), `frontend/src/types.ts`.
- Tests: `backend/src/updates/history.rs`, `backend/src/db.rs`, `backend/src/models.rs`; `frontend/src/HistoryPage.test.tsx`, `frontend/src/App.test.tsx`.
- Docs: `docs/update-history-pagination.md` (nuevo) y `HANDOFF.md`.

## Tareas

El checklist TDD (RED → GREEN → REFACTOR, backend y frontend) vive en `openspec/changes/update-history-pagination/tasks.md`.

## Verificación

- Backend: `cd backend && just check` limpio y `cargo test` (baseline 541 tests, no bajar).
- Frontend: `pnpm exec tsc -b`, `pnpm run lint`, `pnpm run test` (baseline 209 tests, no bajar) y `pnpm run build`.
- Auditorías: `@rust-reviewer` (backend) y `@react-reviewer` (frontend), sin hallazgos bloqueantes.
- E2E: `GET /api/history?page=2&page_size=50` con el envelope correcto, `page_size=30` → 400, y la vista paginando con 10/20/50.

Resultados medidos: backend `cargo test` → 576 passed / 0 failed (baseline 541) y `just check` limpio; frontend `pnpm run test` → 233 passed / 0 failed (baseline 209), con `tsc -b`, `lint` y `build` limpios, y la suite verificada estable en 5 ejecuciones consecutivas bajo carga de CPU. El cap de 100 se elimina al servir; `load_update_history` sigue truncando a 100 filas al arrancar, pero solo alimenta el vector en memoria (`export_config_h` y los `push` de los workers), no la vista.

## Riesgos

- Cambio incompatible del endpoint (array → envelope); no hay consumidores conocidos fuera de `HistoryPage`.
- Coste creciente de `COUNT(*)` y `LIMIT/OFFSET` sobre una tabla sin poda (mitigado por el índice nuevo; la poda queda como follow-up).
- Reescritura de los 8 tests de `history.rs`, con riesgo de perder cobertura de `delete_history_h` si no se migran con cuidado.

## Follow-ups

- Poda o retención de `update_history`.
- Migrar `ContainerRow` e `InspectModal` a `EllipsisText`.
- Tooltip accesible por teclado (deuda ya anotada en `docs/inspect-modal-image.md`).
