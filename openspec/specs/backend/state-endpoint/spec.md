# backend/state-endpoint Specification

## Purpose

Capability del backend que expone el endpoint unificado de estado `GET /api/state`,
que devuelve un envelope `StateResponse` con los contenedores, el resumen agregado
(`ContainerSummary`) y el progreso de actualizaciones en una sola respuesta.
El endpoint usa long-poll sobre `state_notify` (`tokio::sync::Notify`) para despertar
a los clientes en cuanto cambia el estado, y `compute_summary` deriva las cuentas
de contenedores por estado.

## Requirements

### Requirement: El endpoint `GET /api/state` SHALL devolver un envelope `StateResponse`

El handler `state_h` SHALL devolver `200 OK` con JSON `StateResponse { containers,
summary, progress }`, donde `summary` se calcula con `compute_summary(containers)`
y `progress` proviene del `progress_cache`. La respuesta SHALL incluir el header
`Cache-Control: no-cache`.

#### Scenario: Petición de estado con cache poblada

- **GIVEN** un cliente que llama a `GET /api/state`
- **WHEN** `cached_containers` y `progress_cache` ya están poblados
- **THEN** el handler responde `200 OK` con `StateResponse`
- **AND** la respuesta incluye `Cache-Control: no-cache`

### Requirement: El endpoint SHALL hacer long-poll con timeout de 30 segundos

Antes de leer la cache, el handler SHALL esperar en `state_notify.notified()` envuelto
en `tokio::time::timeout(Duration::from_secs(30))`; si expira el timeout sin
notificación, SHALL devolver igualmente el estado actual de la cache.

#### Scenario: Sin cambios durante 30 segundos

- **GIVEN** un cliente que está en long-poll sobre `GET /api/state`
- **WHEN** no ocurre ningún cambio de estado ni de progreso en 30 segundos
- **THEN** el handler devuelve la cache actual tras expirar el timeout
- **AND** el frontend recibe la respuesta y vuelve a hacer poll inmediatamente

#### Scenario: Cambio de estado despierta al handler

- **GIVEN** un cliente que está en long-poll sobre `GET /api/state`
- **WHEN** un contenedor se detiene y el state worker actualiza la cache llamando a `state_notify.notify_waiters()`
- **THEN** el handler en espera despierta, lee la cache actualizada y devuelve `StateResponse`

### Requirement: Los productores de estado SHALL notificar vía `state_notify`

`workers/state.rs` SHALL llamar a `state_notify.notify_waiters()` tras actualizar la
cache en `refresh()`, y `updates/handlers.rs` SHALL llamar a
`state_notify.notify_waiters()` tras insertar en la cache en `update_progress()`.

#### Scenario: Progreso de batch actualizado

- **GIVEN** un cliente que está en long-poll sobre `GET /api/state`
- **WHEN** `update_progress()` se invoca durante una operación por lotes
- **THEN** actualiza `progress_cache` y llama a `state_notify.notify_waiters()`
- **AND** el handler en espera despierta y devuelve `StateResponse` con el progreso actualizado

### Requirement: `compute_summary` SHALL calcular los conteos de contenedores

La función `compute_summary(containers: &[ContainerInfo])` SHALL devolver un
`ContainerSummary` con los campos `total`, `running`, `stopped`, `paused` y
`with_updates` correctamente contabilizados.

#### Scenario: Resumen de una lista heterogénea

- **GIVEN** una lista de contenedores en distintos estados
- **WHEN** se llama a `compute_summary()`
- **THEN** devuelve los conteos correctos de `total`, `running`, `stopped`, `paused` y `with_updates`

### Requirement: El endpoint SHALL esperar la primera notificación cuando la cache está vacía

Si `cached_containers` es `None` (aún no poblada en el primer arranque), el handler
SHALL esperar la primera notificación del state worker antes de responder.

#### Scenario: Primera llamada con cache vacía

- **GIVEN** un cliente que llama a `GET /api/state`
- **WHEN** `cached_containers` es `None` porque aún no se ha poblado
- **THEN** el handler espera la primera notificación del state worker
- **WHEN** el state worker envía su primer refresh
- **THEN** el handler devuelve la cache ya poblada
