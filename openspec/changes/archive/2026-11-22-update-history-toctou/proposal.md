# Proposal

## Why

`get_history_h` (`backend/src/updates/history.rs`) resuelve el envelope paginado con **dos sentencias separadas** sobre la misma conexión: `db::count_update_history` y después `db::load_update_history_page`. En SQLite cada sentencia en autocommit es su propia transacción, así que una escritura concurrente (`append_update_history`, que ejecutan los workers y el scheduler) puede commitear **entre** ambas y dejar la respuesta internamente incoherente: `total` cuenta la fila nueva y `items` no la ve, o al revés.

El caso se manifiesta justo donde importa: en la última página, con `page_size` mayor o igual que el total, el cliente recibe `total` = N+1 con N elementos. El frontend no puede detectar el desajuste porque confía en `total` para calcular el número de páginas, de modo que puede mostrar una página vacía o perder la fila recién insertada hasta el siguiente refresco. Es un TOCTOU clásico de paginación, ya documentado como follow-up en `docs/update-history-pagination.md`.

Issue #208.

## What Changes

- Nueva función en `backend/src/db.rs` que resuelve **recuento y ventana en la misma transacción de lectura** (`BEGIN DEFERRED` vía `Connection::transaction()`), devolviendo `(total, items)` en una sola llamada: las dos sentencias ven la misma instantánea del WAL.
- `get_history_h` pasa a usarla en lugar de las dos llamadas actuales; el envelope `{items, total, page, page_size}` y el mapeo de errores (`AppError::from_db`) no cambian.
- `db::count_update_history` y `db::load_update_history_page` se conservan (los usan tests y `models.rs` los documenta); la nueva función los reutiliza por dentro o replica su `WHERE` a partir de `update_history_filter`, sin duplicar la lógica de filtros.
- Se mantiene la propiedad de que el handler **no** toma `update_history.lock()`: la consistencia se resuelve en SQLite, no serializando contra los workers.
- Test nuevo en `backend/src/db.rs` con DB en fichero temporal y pool de 2 conexiones: un hilo hace `append_update_history` en bucle mientras otro lee páginas con `page_size` >= total; en cada iteración `total` MUST ser igual a `items.len()`. Test de regresión del caso `offset` más allá del final (`items` vacío, `total` correcto).
- Sin cambios en frontend ni en el contrato HTTP.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `backend/update-history-pagination`: se añade el requirement de consistencia de instantánea (`total` e `items` desde la misma lectura) con sus escenarios, incluido el de `offset` más allá del final y el de append concurrente.

## Impact

- Código: `backend/src/db.rs` (nueva función + tests), `backend/src/updates/history.rs` (handler).
- Sin cambios de esquema, migraciones ni contrato de API.
- Riesgo de contención: una transacción de lectura en WAL no bloquea a los escritores; el único coste es retener la conexión de la pool durante ambas sentencias, que ya ocurría hoy.
- Frontend: sin cambios (mismo envelope).