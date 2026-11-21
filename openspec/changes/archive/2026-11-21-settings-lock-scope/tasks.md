# Tasks — settings-lock-scope

## 1. RED (test que falla)

- [x] 1.1 Backend: test determinista del invariante — pool de tamaño 1 ocupada por el test, lanzar
      `update_config_h` y comprobar con `tokio::time::timeout` que `settings.lock()` sigue siendo
      adquirible mientras el handler espera conexión.
- [x] 1.2 Backend: invariante replicado en `admin.rs` (`put_default_update_policy_h`) y, tras la
      auditoría, también en `put_update_check_config_h`, `export_config_h` (comprueba además
      `update_history.lock()`) e `import_config_h`.
- [x] 1.3 `cargo test`: 1.1/1.2 FALLAN con el código antiguo (RED: 459 passed / 2 failed) y el resto verde.

## 2. GREEN (implementación mínima)

- [x] 2.1 `config.rs::update_config_h`: conn antes del `settings.lock()` (+ `drop(conn)` antes de
      `config_handler(...)`).
- [x] 2.2 `admin.rs::put_update_check_config_h`: ídem.
- [x] 2.3 `admin.rs::put_default_update_policy_h`: ídem.
- [x] 2.4 `admin.rs::export_config_h`: conn antes de los locks.
- [x] 2.5 `admin.rs::import_config_h`: conn antes del `settings.lock()`.
- [x] 2.6 `cargo test`: 464 passed / 12 ignored / 0 failed.

## 3. REFACTOR y verificación

- [x] 3.1 Helper de test compartido (`test_support.rs`) y endurecido (`yield_now` + `!is_finished()`).
- [x] 3.2 `cd backend && just check` (fmt + clippy `-D warnings`) limpio.
- [x] 3.3 `cd backend && cargo test` completo en verde (464 OK, 12 ignored).

## 4. Correcciones de auditoría (@rust-reviewer)

- [x] 4.1 Hallazgo 2: 3 tests añadidos para completar el escenario del spec.
- [x] 4.2 Hallazgo 3: helper sin falso verde.
- [x] 4.3 Hallazgo 4: `design.md` alineado con el código.
- [x] 4.4 Hallazgo 5: `drop(conn)` en `update_config_h`.
- [x] 4.5 Hallazgo 1: fuera de alcance → issue **#170** (orden pool↔`update_history` en `updates/handlers.rs`).

## 5. Cierre

- [x] 5.1 Auditoría final `@rust-reviewer` (veredicto apto).
- [x] 5.2 `docs/settings-lock-scope.md`.
- [x] 5.3 PR de `feature/settings-lock-scope` a `develop`, CI verde.
- [x] 5.4 `openspec archive settings-lock-scope` + limpieza del change.
- [x] 5.5 Cerrar issue #158 y renombrar este plan a `plans/PLAN-014-ARCHIVED.md`; actualizar `HANDOFF.md`.
