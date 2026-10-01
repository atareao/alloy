# Spec Delta

## Purpose

Endurecer la fiabilidad del worker `update-check`: que ningún helper de persistencia pueda
paniquear y matar la tarea, y que la política `Pull` no re-descargue una imagen en modo
degradado en cada ciclo.

## ADDED Requirements

### Requirement: Los helpers de persistencia del scheduler no paniquean

Los helpers de acceso a la base de datos y a locks usados por el worker `update-check` MUST
NOT usar `unwrap()` sobre `db_pool.get()` ni sobre `mutex.lock()`. Ante pool cerrado o
mutex envenenado MUST registrar `ERROR` (o `WARN` cuando proceda) y continuar, sin panic.

#### Scenario: Pool de base de datos cerrado

- **Given** un `db_pool.get()` que devuelve `Err` (pool agotado o cerrado)
- **When** un helper de persistencia (`update_has_update`, `append_update_history`,
  `check_times`, `save_settings`) intenta obtener conexión
- **Then** se registra un `ERROR` con el contexto
- **And** no se produce panic
- **And** el ciclo del worker continúa con el siguiente contenedor

#### Scenario: Mutex envenenado

- **Given** un mutex de conexión envenenado (poisoned)
- **When** un helper intenta `lock()`
- **Then** se registra un `ERROR` y no se produce panic

#### Scenario: Fallo al cargar el mapa de digests

- **Given** que `load_last_remote_digest_map` no puede preparar la consulta o el lock
- **When** se carga el mapa al inicio del ciclo
- **Then** se devuelve un mapa vacío (comportamiento actual para prepare/query)
- **And** el fallo del lock también se registra sin panic

### Requirement: La política Pull no repite la descarga degradada

Cuando la resolución remota es degradada, la política `Pull` MUST NOT volver a descargar la
misma imagen en ciclos sucesivos mientras el manifest no cambie. Esta supresión MUST NOT
persistir `last_remote_digest` (la semántica de `Pull` de no avanzar el digest se mantiene).

#### Scenario: Pull degradado aplicado una vez

- **Given** un contenedor con política `Pull` y resolución degradada
- **When** se aplica el pull en un ciclo
- **Then** en el siguiente ciclo, con el mismo manifest degradado, no se vuelve a descargar
- **And** `last_remote_digest` NO se persiste

#### Scenario: Manifest degradado nuevo

- **Given** un contenedor con un manifest degradado ya aplicado
- **When** el manifest remoto degradado cambia
- **Then** se vuelve a descargar (no se suprime)

#### Scenario: Recuperación del registry

- **Given** un contenedor cuya resolución pasa de degradada a fiable
- **When** se ejecuta el ciclo
- **Then** la supresión no aplica y se recalcula `needs_update` con el config digest fiable
