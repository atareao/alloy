# updates/pull-cache-digest Specification

## Purpose
Evitar que la política `Pull` descargue la misma imagen en cada ciclo de comprobación,
registrando el digest de la imagen ya presente en el caché local y omitiendo el pull
cuando el remoto no ha cambiado.

## Requirements

### Requirement: Persistir el digest descargado al caché

El sistema MUST registrar por contenedor el config digest de la última imagen descargada
al caché local (`last_pulled_digest`), independientemente del digest de la imagen en
ejecución (`last_remote_digest`).

#### Scenario: Pull exitoso persiste el digest

- **Given** un contenedor con política `Pull` y `last_pulled_digest` vacío
- **And** el remoto resuelve `config_digest=sha256:bbb`
- **When** el pull termina con éxito
- **Then** `last_pulled_digest=sha256:bbb`

#### Scenario: Pull fallido no persiste

- **Given** un contenedor con `last_pulled_digest=sha256:aaa`
- **When** el pull falla
- **Then** `last_pulled_digest` sigue `sha256:aaa`

### Requirement: Omitir el pull si la imagen ya está en caché

Con la política `Pull`, si el config digest remoto coincide con `last_pulled_digest`, el
sistema MUST omitir la descarga.

#### Scenario: Imagen ya en caché

- **Given** un contenedor con política `Pull` y `last_pulled_digest=sha256:bbb`
- **And** el remoto resuelve `config_digest=sha256:bbb`
- **When** se ejecuta el check
- **Then** NO se realiza pull
- **And** no se registra error

#### Scenario: Imagen nueva en el remoto

- **Given** un contenedor con política `Pull` y `last_pulled_digest=sha256:aaa`
- **And** el remoto resuelve `config_digest=sha256:bbb`
- **When** se ejecuta el check
- **Then** se realiza pull
- **And** `last_pulled_digest` pasa a `sha256:bbb`

#### Scenario: El skip es un no-op

- **Given** un contenedor con política `Pull` y `last_pulled_digest=sha256:bbb`
- **And** el remoto resuelve `config_digest=sha256:bbb`
- **When** se ejecuta el check
- **Then** NO se envía notificación de actualización
- **And** NO se añade entrada al historial de updates
- **And** NO se ejecuta prune de imágenes dangling

### Requirement: has_update no cambia con la política Pull

`has_update` MUST seguir indicando si el contenedor ejecuta la imagen remota. Con la
política `Pull` (sin recreate) permanece `true` aunque la imagen ya esté en caché.

#### Scenario: Pull no limpia has_update

- **Given** un contenedor con política `Pull` y update disponible
- **When** el pull termina con éxito
- **Then** `has_update` sigue `true`
- **And** `last_remote_digest` NO avanza

### Requirement: Migración idempotente

La migración que añade `last_pulled_digest` MUST ser idempotente y no perder datos.

#### Scenario: Migración sobre BD existente

- **Given** una BD con la tabla `containers` sin `last_pulled_digest`
- **When** arranca Alloy
- **Then** la columna se añade con default `''`
- **And** los contenedores existentes conservan sus datos
