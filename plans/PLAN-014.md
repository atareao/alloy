# PLAN-014 — settings-lock-scope

- **Issue:** #158 — perf(backend): no retener `settings.lock()` durante `db_pool.get().await`
- **Change OpenSpec:** `settings-lock-scope` (proposal + specs/backend/concurrency/spec.md + design.md + tasks.md)
- **Rama:** `feature/settings-lock-scope` (desde `development`)
- **Estado:** planificado

## Objetivo

Que ningún handler de persistencia de configuración retenga el mutex de `Settings` (ni el de
`update_history`) mientras espera una conexión de la pool de BD. Eliminar la serialización de
todas las lecturas/escrituras de settings durante `db_pool.get().await`.

## Alcance

Handlers afectados:
- `backend/src/config.rs::update_config_h` (PUT /api/config)
- `backend/src/admin.rs::put_update_check_config_h`
- `backend/src/admin.rs::put_default_update_policy_h`
- `backend/src/admin.rs::export_config_h`
- `backend/src/admin.rs::import_config_h`

Decisión: orden uniforme **conexión antes que estado** (`db_pool.get()` + `conn.lock()` antes de
`settings.lock()`). Sin cambios de API ni de datos. Riesgo bajo; se evita deadlock manteniendo el
mismo orden en todos los handlers.

## Tareas (TDD)

1. **RED**: test determinista del invariante (pool de tamaño 1 ocupada por el test; el handler no
   debe bloquear `settings.lock()` mientras espera conexión), con `tokio::time::timeout`. Aplicar a
   `update_config_h` y `put_default_update_policy_h`. Confirmar que fallan con el código actual.
2. **GREEN**: reordenar la adquisición en los 5 handlers. `cargo test` en verde.
3. **REFACTOR**: helper uniforme si procede; `cd backend && just check` (fmt + clippy -D warnings);
   `cargo test` completo (baseline 459 OK, 12 ignored).
4. **Cierre**: auditar con @rust-reviewer; crear `docs/settings-lock-scope.md`; PR a `development`;
   CI verde; `openspec archive settings-lock-scope` + limpieza; cerrar issue #158; renombrar este
   plan a `plans/PLAN-014-ARCHIVED.md` y actualizar `HANDOFF.md`.

## Verificación

- Backend: `cd backend && cargo test` (459 OK), `just check` limpio.
- Sin cambios en frontend.
