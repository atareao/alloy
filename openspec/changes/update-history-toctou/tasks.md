# Tasks

## 1. RED — tests que fijan la consistencia

- [x] 1.1 `backend/src/db.rs`: test que llama a la nueva lectura (`(total, items)` en una transacción) sobre 25 entradas con `page_size` 50 y afirma `total == items.len() == 25`.
- [x] 1.2 Test de regresión: `page = 99` con 25 entradas devuelve `items` vacío y `total` 25.
- [x] 1.3 Test de concurrencia: DB en fichero temporal (`tempfile`) con pool de 2; un hilo hace `append_update_history` en bucle acotado mientras el test lee páginas con `page_size` 50 en bucle; en cada lectura `total` MUST igualar `items.len()`. Ejecutar `cargo test` y confirmar que falla sin la transacción (RED) y pasa con ella.
- [x] 1.4 Verificar que los tests actuales de `backend/src/updates/history.rs` siguen compilando sin cambios de contrato.

## 2. GREEN — lectura transaccional

- [x] 2.1 Añadir en `backend/src/db.rs` la función que abre `conn.transaction()` (deferred), ejecuta recuento y ventana con el mismo `update_history_filter` y devuelve `(usize, Vec<UpdateHistoryEntry>)`.
- [x] 2.2 `get_history_h` usa la nueva función; se elimina la doble llamada. Sin cambios en el envelope ni en el mapeo de errores.
- [x] 2.3 `cargo test` verde (suite completa) y `cd backend && just check` limpio.

## 3. REFACTOR y documentación

- [x] 3.1 Revisar que `count_update_history` / `load_update_history_page` no quedan como código muerto ni duplicado; documentar la nueva función con `///` explicando la instantánea.
- [x] 3.2 Actualizar `docs/update-history-pagination.md`: el follow-up del TOCTOU pasa a "resuelto" con el mecanismo (transacción de lectura) y la fecha.
- [x] 3.3 `openspec validate --strict --specs` y `openspec validate --changes` en verde.