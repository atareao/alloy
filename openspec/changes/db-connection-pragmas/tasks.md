# Tasks

## 1. RED

- [x] 1.1 Test en `backend/src/db.rs`: `test_pool()` → tomar dos conexiones distintas y afirmar que `PRAGMA busy_timeout` devuelve 5000 y `PRAGMA foreign_keys` devuelve 1 en ambas. Debe fallar con el código actual (0 y 0).
  - Nota: `test_pool_two_connections` se conserva como guarda de regresión, pero **no es un RED válido** en este build: `rusqlite` 0.38 fija `busy_timeout=5000` en `Connection::open` y `libsqlite3-sys` compila con `-DSQLITE_DEFAULT_FOREIGN_KEYS=1`, así que pasa sin código propio. El RED válido es `test_configure_connection_applies_pragmas`.
- [x] 1.2 Test de conexión reciclada: tomar, soltar y volver a tomar la conexión de una pool de tamaño 1 y afirmar `busy_timeout` = 5000.
  - Nota: igual que 1.1, `test_pool_recycle` pasa por los defaults de la crate (guarda de regresión).
- [x] 1.3 Test de `init_db` sobre `tempfile::tempdir()`: tras inicializar, `PRAGMA journal_mode` devuelve `wal`, y una segunda llamada a `init_db` sobre el mismo fichero no falla (idempotencia).
  - Nota: `test_init_db_tempdir_wal` pasa porque `init_db` ya aplicaba WAL; guarda de regresión de la idempotencia.
- [x] 1.4 `cargo test` y confirmar RED en 1.1 y 1.2.
  - RED válido confirmado con los tests nuevos que sí dependen del código propio: `test_configure_connection_applies_pragmas`, `test_journal_mode_warning_for_delete`, `test_append_history_best_effort_logs_error`.

## 2. GREEN

- [x] 2.1 `configure_connection(conn: &Connection) -> SqlResult<()>` en `db.rs`: `PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;`.
- [x] 2.2 `PragmaManager` que envuelva `deadpool_sqlite::Manager`, delegue `create` aplicando `configure_connection` antes de devolver la conexión, y delegue `recycle`. `DbPool` pasa a `Pool<PragmaManager>`.
  - `recycle` además **reaplica** `configure_connection` a la conexión reciclada (defensa en profundidad); si falla, devuelve `RecycleError` y deadpool la descarta.
  - Limitación de test del wiring de `create`: como los defaults de `rusqlite`/`libsqlite3-sys` ya valen 5000/1, `test_pool_two_connections` NO falla si se elimina el `interact(configure_connection)` de `create`. La lógica de `configure_connection` la cubre `test_configure_connection_applies_pragmas`, y el wiring de `recycle` lo cubre `test_pool_recycle_reapplies_pragmas`.
- [x] 2.3 `init_db` y `pool_with_schema` construyen la pool con el nuevo manager; `init_db` verifica el modo journal devuelto y avisa con `tracing::warn!` si no es `wal`.
- [x] 2.4 `backend/src/workers/scheduler.rs`: sustituir `let _ =` por el helper `append_history_best_effort` que loguea el error con contenedor e imagen.
- [x] 2.5 Suite completa verde y `just check` limpio.

## 3. Verificación y cierre

- [x] 3.1 Comprobar que `main.rs` (`pool.close()`) y los ~60 `pool.get()` siguen compilando sin cambios.
- [x] 3.2 Actualizar `docs/architecture.md` con una nota sobre PRAGMAs por conexión y verificación de WAL.
- [x] 3.3 `openspec validate --strict --specs` y `openspec validate --changes` en verde.
