# Alcance de los locks de Settings (#158)

## Qué es

Refactor **de concurrencia** (sin cambio de comportamiento observable de la API) en los handlers de
persistencia de configuración del backend. Antes, varios handlers tomaban `settings.lock().await` y,
**con el guard vivo**, llamaban a `db_pool.get().await`. Mientras esperaban una conexión de la pool
(que puede alargarse si está saturada) retenían el mutex de `Settings`, serializando **todas** las
lecturas/escrituras de configuración y acoplando su latencia a la de la BD.

Cambio OpenSpec: `settings-lock-scope` (capability `backend/concurrency`, archivado en
`openspec/specs/backend/concurrency/spec.md`).
Plan: `plans/PLAN-014-ARCHIVED.md`. Issue: #158.

## Ficheros implicados

- `backend/src/config.rs` — `update_config_h` (`PUT /api/config`).
- `backend/src/admin.rs` — `put_update_check_config_h`, `put_default_update_policy_h`,
  `export_config_h`, `import_config_h`.
- `backend/src/db.rs` — helpers de test `pool_with_schema()` / `test_pool_with_max_size(max_size)`.
- `backend/src/test_support.rs` — helper `assert_settings_lock_free_while_db_pool_saturated`.
- Tests colocalizados en `config.rs` y `admin.rs`.

## Cómo funciona

Regla de orden uniforme **"conexión antes que estado"**:

```rust
let conn = db_pool.get().await.unwrap();   // se espera la pool SIN ningún guard de Settings
let guard = conn.lock().unwrap();          // guard síncrono de rusqlite, sección sin await
let mut s = settings.lock().await;         // ahora sí, el mutex de estado
// ... mutar s ...
let _ = db::save_settings(&guard, &s);     // persistencia dentro de la misma sección crítica
```

- La sección crítica (mutar `s` + `save_settings`) es **síncrona**; el único `await` con estado ya no
  depende de la pool.
- El orden de adquisición es **idéntico en todos los handlers** (pool → settings), lo que descarta
  ciclos de espera entre ambos recursos.
- Sin cambios de contrato: mismas rutas, mismo JSON, misma validación de cron. `export_config_h`
  conserva su persistencia y su redacción de secretos.

## Variables / config

Ninguna. No hay flags ni variables de entorno; es puramente interno.

## Cómo verificar / desplegar

- Tests: `cd backend && cargo test` (461 OK / 12 ignored). Los dos tests del invariante son
  `config::tests::test_update_config_does_not_hold_settings_lock_while_waiting_for_db` y
  `admin::tests::test_put_default_update_policy_does_not_hold_settings_lock_while_waiting_for_db`.
- Lint/format: `cd backend && just check`.
- Sin migraciones ni pasos de despliegue: basta con reconstruir la imagen habitual.

## Decisiones

- **Conexión antes que estado**, en lugar de "soltar el guard y clonar `Settings`": evita clonar una
  struct grande en cada PUT y evita escribir un snapshot potencialmente obsoleto.
- Se mantiene el `unwrap()` preexistente sobre `db_pool.get()` (no se añade manejo de error nuevo:
  fuera del alcance del issue).
- La cobertura se limita a `update_config_h` y `put_default_update_policy_h` (representativos); el
  helper es reutilizable para cubrir el resto si se desea.

## Follow-ups

- Opcional: extender el test del invariante a `put_update_check_config_h`, `export_config_h` e
  `import_config_h` con el helper ya existente (bajo coste).
