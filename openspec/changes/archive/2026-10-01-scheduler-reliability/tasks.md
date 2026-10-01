# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 Helper puro de decisión de supresión de pull degradado (`pull_degraded_suppressed`):
      suprime con mismo manifest aplicado, no suprime con manifest nuevo ni al recuperarse.
- [x] 1.2 Test de forma (grep-asert) que verifica que los helpers de persistencia de
      `scheduler.rs` no contienen `db_pool.get().await.unwrap()` ni `lock().unwrap()`.
- [x] 1.3 `cargo test` en rojo con los legacy en verde.

## 2. GREEN — Implementación mínima

- [x] 2.1 `sqlite_update_has_update` y `sqlite_append_update`: sustituir `.unwrap()` por
      manejo `if let Ok(..)` + `ERROR` (y evitar `hist.last().unwrap()`).
- [x] 2.2 Bucle de `update_container_check_times` y `save_settings`: idem.
- [x] 2.3 `load_last_remote_digest_map`: `conn.lock()` y `db_pool.get()` sin panic.
- [x] 2.4 Rama `UpdateAction::Pull`: alimentar el guard `degraded_applied` (sin persistir
      `last_remote_digest`) para suprimir pulls degradados repetidos.
- [x] 2.5 `persist_last_remote_digest_after_recreate`: bajar el `WARN` duplicado a `info!`.
- [x] 2.6 `cargo test` verde + `cargo check`.

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 `cargo fmt`
- [x] 3.2 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 `cargo test` sin regresiones
- [x] 3.4 Revisión con `rust-reviewer`

## 4. Archive

- [x] 4.1 `openspec validate scheduler-reliability --strict`
- [x] 4.2 `openspec archive scheduler-reliability --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/scheduler-reliability/`
