# PLAN-033 — Completar PRAGMAs por conexión y verificación WAL (#210)

- **Issue:** #210 (reabierto)
- **Change:** `openspec/changes/db-connection-pragmas/` (aprobado)
- **Rama:** `feature/db-connection-pragmas-v2` (desde `develop`)
- **Fecha:** 2026-11-22

## Objetivo

Completar la implementación GREEN que el PR #211 no llegó a mergear: PRAGMAs de sesión en cada conexión de la pool, verificación del modo journal con aviso y log de error del append del histórico.

## Hallazgo clave (false-green)

El PR #211 (merge `6359f26`) solo aportó el commit `d82e48d`, que añadió **tests** y el change proposal, no la implementación. Los tests pasan por los defaults de las crates:

- `rusqlite` 0.38 fija `busy_timeout=5000` en `Connection::open` (`inner_connection.rs:118`).
- `libsqlite3-sys` compila con `-DSQLITE_DEFAULT_FOREIGN_KEYS=1`.

Por eso `test_pool_two_connections` no puede fallar en este build. Hay que sustituir el RED por tests que ejerzan la lógica que falta.

## Alcance

- `backend/src/db.rs`: `configure_connection`, `PragmaManager` (envuelve `deadpool_sqlite::Manager`), `DbPool` como `Pool<PragmaManager>`, verificación del modo journal en `init_db`.
- `backend/src/workers/scheduler.rs`: sustituir `let _ = append_update_history(...)` por log de error con contenedor e imagen.
- Tests: `configure_connection` sobre conexión con PRAGMAs desactivados; `journal_mode_warning("delete")`/`("wal")`; append best-effort sobre conexión read-only con captura de `tracing::error!`.
- `docs/architecture.md`: nota sobre PRAGMAs por conexión y verificación WAL.
- Sin cambios de contrato HTTP, esquema ni frontend.

## Pasos (TDD)

1. [ ] RED: stubs + tests que fallen por la razón correcta; `cargo test` con el resto en verde.
2. [ ] GREEN: implementar `configure_connection`, `PragmaManager`, verificación WAL y log del scheduler; `cargo test` verde.
3. [ ] REFACTOR: `cargo fmt`, `cargo clippy -- -D warnings`, `just check`; suite completa.
4. [ ] Auditoría `@rust-reviewer`; corregir hallazgos.
5. [ ] Docs: `docs/architecture.md`; validar `openspec validate --strict --specs` y `--changes`.
6. [ ] Commit gitmoji + PR a `develop`; cerrar #210 a mano tras el merge.
7. [ ] Archivar el change y limpiar `openspec/changes/db-connection-pragmas/`.

## Criterios de aceptación

- [ ] `configure_connection` aplica `busy_timeout=5000` y `foreign_keys=ON` a una conexión que los tenía a 0/OFF.
- [ ] `PragmaManager` aplica `configure_connection` en `create` y delega `recycle`; `DbPool` compila con los ~60 `pool.get()`.
- [ ] `init_db` lee el modo devuelto y emite `tracing::warn!` accionable si no es `wal`.
- [ ] El append best-effort del scheduler registra `tracing::error!` con contenedor e imagen y no silencia el `Result`.
- [ ] `cargo test` verde, `just check` limpio, `openspec validate` verde.
- [ ] Sin cambios de contrato HTTP.