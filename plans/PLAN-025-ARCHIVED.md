# PLAN-025 — Orden de locks de update_history (#170)

**Issue:** #170
**Change OpenSpec:** `openspec/changes/update-history-lock-order/`
**Rama:** `feature/update-history-lock-order` (desde `develop`)

## Objetivo

Eliminar el riesgo de ciclo entre el permiso de la pool y el guard de `update_history` en los helpers que aún retienen el guard durante `db_pool.get().await`. El issue apunta a `handlers.rs`, pero el análisis muestra que ese fichero ya está resuelto; el riesgo real está en `stacks.rs` y `history.rs`.

## Mapa de cambios

- `backend/src/stacks.rs`: `append_stack_history` suelta el guard antes de la pool.
- `backend/src/updates/history.rs`: `delete_history_h` suelta el guard antes de la pool.
- `backend/src/updates/common.rs`: eliminar `record_update_entry` (código muerto).
- `backend/src/test_support.rs`: helper de test de no-bloqueo (si no existe uno adecuado).
- Tests en `stacks.rs` y `history.rs`.
- `docs/update-history-lock-order.md`.

## Tareas

1. Añadir tests de no-bloqueo con pool `max_size(1)` saturada.
2. Refactorizar `append_stack_history` y `delete_history_h` con el patrón `estado → drop → pool`.
3. Eliminar el código muerto de `common.rs`.
4. Verificar `cargo test` y `just check`.
5. Documentar y preparar el PR.

## Criterios de aceptación

- [x] Ningún guard de `update_history` cruza el `await` de la pool fuera de `push_history_entry`.
- [x] `record_update_entry` eliminado.
- [x] Tests de no-bloqueo en verde.
- [x] `cargo test` ≥539 tests y `just check` en 0.
- [x] Diff limitado a `backend/` + artefactos OpenSpec/plan/docs.
