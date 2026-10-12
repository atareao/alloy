# Proposal

## Why

Dos hallazgos MENOR de las auditorías de `update-check-robustness` quedaron pendientes:

1. **`unwrap()` en paths de producción del scheduler.** `workers/scheduler.rs` usa
   `db_pool.get().await.unwrap()` y `conn.lock().unwrap()` en helpers que corren dentro del
   worker `tokio::spawn`: `sqlite_update_has_update` (l.731-732), `sqlite_append_update`
   (l.759-760, incl. `hist.last().unwrap()`), el bucle de `update_container_check_times`
   (l.491-493), `save_settings` (l.506) y `load_last_remote_digest_map` (l.772). Un pool
   agotado o un mutex envenenado **paniquea y mata la tarea del worker**, deteniendo el
   update-check hasta el próximo reinicio.

2. **La política `Pull` repite la descarga en modo degradado.** Cuando el registry no
   expone el config digest (degradado), `Pull` no avanza `last_remote_digest` (correcto)
   pero tampoco alimenta el guard anti-bucle, así que `needs_update` vuelve a ser `true`
   cada ciclo y **re-descarga la misma imagen indefinidamente**.

## What Changes

- Sustituir los `unwrap()` de los helpers de persistencia del scheduler por manejo
  explícito de error con `tracing::error!`, sin panic y sin matar la tarea.
- Extender el guard anti-bucle degradado (`degraded_applied`) a la política `Pull`, de
  modo que no re-descargue el mismo manifest en ciclos sucesivos **sin** persistir
  `last_remote_digest` (la semántica de `Pull` no cambia).
- Bajar el `WARN` duplicado de `persist_last_remote_digest_after_recreate` a `info!` para
  no duplicar la alarma de degradado.

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `updates/update-check`: se añaden requisitos de robustez de persistencia y de no
  repetición de pull degradado.

## Impact

- `backend/src/workers/scheduler.rs`: helpers de persistencia, bucle de check-times,
  `load_last_remote_digest_map`, rama `UpdateAction::Pull` y guard degradado.
- Sin cambios de frontend, Dockerfile ni schema SQLite.

Fuera de alcance: NITs de `stacks.rs` (doble filtrado de redes, `services` vacío) y clones
en `resolve_digest_cached`; se pueden abordar en otro change si se desea.
