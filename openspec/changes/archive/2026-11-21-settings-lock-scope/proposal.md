# Proposal

## Why

Varios handlers toman `settings.lock().await` y, **con el guard vivo**, llaman a
`db_pool.get().await` (y a `db::save_settings`). Mientras esperan una conexión de la pool
(puede alargarse si está saturada) retienen el mutex de `Settings`, serializando **todas**
las lecturas/escrituras de configuración: escalado pobre y latencia acoplada a la BD.
Referencia: issue #158.

## What Changes

- Reordenar los handlers de persistencia de configuración para **adquirir la conexión de la
  pool antes del `settings.lock()`** (o soltar el guard antes del `await`), de modo que ningún
  guard del mutex de `Settings` cruce un `.await` de la pool.
- Afecta a: `PUT /api/config` (`config.rs::update_config_h`), y en `admin.rs`
  `put_update_check_config_h`, `put_default_update_policy_h`, `export_config_h`,
  `import_config_h`.
- Sin cambios de comportamiento observable en la API ni en el formato de datos: la mutación
  y el guardado siguen siendo atómicos respecto a `Settings`.
- Regresión: si el pool está saturado, el handler ya no bloquea a otros lectores de `Settings`.

## Capabilities

### New Capabilities

- `backend/concurrency`: garantía de que ningún handler retiene el mutex de `Settings` (ni el
  de `update_history`) mientras espera a la pool de BD.

### Modified Capabilities

_(ninguna: no cambian requisitos de comportamiento de API existentes)_

## Impact

- **Backend**: `backend/src/config.rs` (`update_config_h`), `backend/src/admin.rs`
  (`put_update_check_config_h`, `put_default_update_policy_h`, `export_config_h`,
  `import_config_h`).
- **API**: sin cambios de contrato (mismas rutas, mismo JSON).
- **Dependencias**: ninguna.
- **Riesgo**: bajo — refactor de alcance de locks; riesgo de deadlock evitado fijando un
  **orden uniforme** (conexión primero, settings después) en todos los handlers afectados.
