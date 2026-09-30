# Proposal

## Why

Con la política `Pull` (descargar sin recrear), Alloy baja la imagen nueva al caché local
pero **no recrea** el contenedor. Como `last_remote_digest` representa la imagen que el
contenedor **ejecuta** y solo avanza tras un recreate, el check sigue viendo
`has_update=true` en cada ciclo y **vuelve a hacer pull de la misma imagen una y otra vez**,
gastando ancho de banda y tiempo sin aportar nada.

## What Changes

- Nueva columna `last_pulled_digest` en `containers`: config digest de la última imagen
  descargada al caché local (independiente de `last_remote_digest`, que es la imagen en
  ejecución).
- En la política `Pull`, si el config digest remoto ya coincide con `last_pulled_digest`,
  se **omite el pull** (la imagen ya está en caché).
- Tras un pull exitoso (cualquier acción que descargue), se persiste
  `last_pulled_digest = remote config digest`.
- `has_update` **no cambia de semántica**: sigue indicando si el contenedor ejecuta o no la
  imagen remota. Con `Pull` permanece `true` (no hubo recreate), pero ya no se re-descarga.

## Capabilities

### New Capabilities
- `updates/pull-cache-digest`: seguimiento del digest descargado al caché para evitar pulls
  redundantes en la política `Pull`.

### Modified Capabilities
<!-- Ninguna: updates/digest-pinned-refs sigue vigente; este cambio añade una capa de caché. -->

## Impact

- `backend/src/db.rs`: migración `last_pulled_digest`, `update_container_last_pulled_digest`,
  `get_container_last_pulled_digest`, `load_last_pulled_digest_map`, `ContainerRow`.
- `backend/src/models.rs`: `ContainerInfo.last_pulled_digest`.
- `backend/src/updates/handlers.rs`: `PendingUpdate`, `apply_single_policy` (rama `Pull`),
  `check_and_apply_all`, `update_container_h`, `update_all_h`.
- Sin cambios de frontend. Migración SQLite idempotente.

Fuera de alcance: cambiar la semántica de `has_update` o el flujo de UI para `Pull`
(seguirá mostrando "update disponible" hasta que el contenedor se recree).
