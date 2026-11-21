# Proposal

## Why

El issue #170 señala que `backend/src/updates/handlers.rs` retiene el guard de `update_history` mientras espera una conexión de la pool. **El análisis del código actual muestra que ese riesgo ya está resuelto en `handlers.rs`**: los cinco puntos citados delegan en `push_history_entry`, que publica en memoria dentro de un bloque y suelta el guard antes de `db_pool.get().await`.

Sin embargo, el mismo patrón inseguro sigue vivo en otros dos sitios:

- `backend/src/stacks.rs:412` (`append_stack_history`): toma `update_history.lock().await`, hace `push` y **mantiene el guard** mientras espera `db_pool.get().await`.
- `backend/src/updates/history.rs:23` (`delete_history_h`): toma `update_history.lock().await`, hace `clear()` y **mantiene el guard** mientras espera `db_pool.get().await`.

Además, `backend/src/updates/common.rs:record_update_entry` conserva el mismo patrón, aunque es código muerto (`#[allow(dead_code)]`, sin call-sites).

Bajo saturación de pool, `export_config_h`/`import_config_h` adquieren la conexión antes que `update_history` (orden `pool → estado`), mientras `append_stack_history`/`delete_history_h` hacen `estado → pool`. Ese orden cruzado puede formar un ciclo (permiso de pool ↔ guard de `update_history`) y bloquear el backend.

## What Changes

- **`stacks.rs::append_stack_history`**: construir el `entry` (ya es un owned `UpdateHistoryEntry`), publicarlo en memoria dentro de un bloque `{}` (soltando el guard), capturar el último elemento clonado y, solo después, esperar la pool y persistir.
- **`history.rs::delete_history_h`**: limpiar el historial en memoria dentro de un bloque `{}` (soltando el guard) y, solo después, esperar la pool y persistir el borrado.
- **`updates/common.rs`**: eliminar `record_update_entry` (código muerto con el patrón inseguro) para que no se reutilice por error. Si se prefiere conservarlo, aplicar el mismo patrón; la opción recomendada es eliminarlo.
- **Tests**: añadir tests de no-bloqueo con pool `max_size(1)` saturada que verifiquen que `update_history.lock()` sigue siendo adquirible mientras `append_stack_history`/`delete_history_h` esperan conexión. Reutilizar el helper de `test_support.rs` o añadir uno específico.
- **Documentación**: `docs/update-history-lock-order.md` con el contrato de orden y la evidencia de que `handlers.rs` ya estaba resuelto.

## Capabilities

### New Capabilities

- `backend/update-history-lock`: garantía de que ningún guard de `update_history` cruza el `await` de la pool en los helpers de historial.

### Modified Capabilities

Ninguna. La capability `backend/concurrency` excluye explícitamente `update_history` y remite a #170.

## Impact

- `backend/src/stacks.rs`, `backend/src/updates/history.rs`, `backend/src/updates/common.rs` (eliminación), `backend/src/test_support.rs` (helper de test si hace falta).
- Tests: `backend/src/stacks.rs`, `backend/src/updates/history.rs`.
- Issue: #170.
