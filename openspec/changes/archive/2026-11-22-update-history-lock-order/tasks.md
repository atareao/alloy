# Tasks

## TDD Checklist

### RED

- [x] Añadir un helper de test en `backend/src/test_support.rs` que compruebe que `update_history.lock()` es adquirible mientras un handler espera una conexión de una pool `max_size(1)` saturada.
- [x] Añadir tests en `backend/src/stacks.rs` para `append_stack_history` con pool saturada (RED: el guard bloquea el lock).
- [x] Añadir tests en `backend/src/updates/history.rs` para `delete_history_h` con pool saturada (RED: el guard bloquea el lock).
- [x] Ejecutar `cargo test` y confirmar los fallos esperados.

### GREEN

- [x] Refactorizar `append_stack_history` para soltar el guard antes de `db_pool.get().await`.
- [x] Refactorizar `delete_history_h` para soltar el guard antes de `db_pool.get().await`.
- [x] Eliminar `record_update_entry` de `backend/src/updates/common.rs`.
- [x] Ejecutar `cargo test` y confirmar que los tests nuevos pasan y los existentes siguen verdes.

### REFACTOR

- [x] Revisar si `append_stack_history` y `delete_history_h` pueden compartir un helper común de publicación + persistencia.
- [x] Ejecutar `cargo clippy -- -D warnings` y `cargo fmt -- --check`.

### Verificación

- [x] `cargo test` → 0 (≥539 tests).
- [x] `just check` → 0.
- [x] Documentar el contrato en `docs/update-history-lock-order.md`.
- [ ] Commit con gitmoji (`⚡ perf`) y PR a `develop`.
