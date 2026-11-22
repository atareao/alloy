# Tasks

## 1. RED

- [ ] 1.1 Test en `backend/src/db.rs`: `test_pool()` → tomar dos conexiones distintas y afirmar que `PRAGMA busy_timeout` devuelve 5000 y `PRAGMA foreign_keys` devuelve 1 en ambas. Debe fallar con el código actual (0 y 0).
- [ ] 1.2 Test de conexión reciclada: tomar, soltar y volver a tomar la conexión de una pool de tamaño 1 y afirmar `busy_timeout` = 5000.
- [ ] 1.3 Test de `init_db` sobre `tempfile::tempdir()`: tras inicializar, `PRAGMA journal_mode` devuelve `wal`, y una segunda llamada a `init_db` sobre el mismo fichero no falla (idempotencia).
- [ ] 1.4 `cargo test` y confirmar RED en 1.1 y 1.2.

## 2. GREEN

- [ ] 2.1 `configure_connection(conn: &Connection) -> SqlResult<()>` en `db.rs`: `PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;`.
- [ ] 2.2 `PragmaManager` que envuelva `deadpool_sqlite::Manager`, delegue `create` aplicando `configure_connection` antes de devolver la conexión, y delegue `recycle`. `DbPool` pasa a `Pool<PragmaManager>`.
- [ ] 2.3 `init_db` y `pool_with_schema` construyen la pool con el nuevo manager; `init_db` verifica el modo journal devuelto y avisa con `tracing::warn!` si no es `wal`.
- [ ] 2.4 `backend/src/workers/scheduler.rs:1032`: sustituir `let _ =` por log de error con contenedor e imagen.
- [ ] 2.5 Suite completa verde y `just check` limpio.

## 3. Verificación y cierre

- [ ] 3.1 Comprobar que `main.rs` (`pool.close()`) y los ~60 `pool.get()` siguen compilando sin cambios.
- [ ] 3.2 Actualizar `docs/architecture.md` con una nota sobre PRAGMAs por conexión y verificación de WAL.
- [ ] 3.3 `openspec validate --strict --specs` y `openspec validate --changes` en verde.