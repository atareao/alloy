# Spec Delta

## Purpose

Definir el stream SSE único de Alloy que entrega estado completo, highlights incrementales, progreso de batch y notificaciones a la UI, manteniendo el long-poll como respaldo.

## ADDED Requirements

### Requirement: El backend SHALL exponer un stream SSE único en `GET /api/stream`

El backend MUST servir `GET /api/stream` como `text/event-stream`, autenticado por cookie de sesión, y MUST multiplexar en una sola conexión los eventos `snapshot`, `container-event`, `update-progress` y `notification`.

**Contracts:**
```rust
pub struct StateResponse { containers: Vec<ContainerInfo>, summary: ContainerSummary, progress: BatchProgress }
pub struct ContainerEvent { name: String, action: String, state: String, timestamp: String }
pub struct UpdateProgress { container: String, status: String, done: bool, error: Option<String>, total: u32, checked: u32, updated: u32, errors: u32 }
pub struct NotifEvent { container: String, status: String, timestamp: String }
```

#### Scenario: Un solo stream sirve todos los eventos

- **GIVEN** un usuario autenticado
- **WHEN** el frontend abre `EventSource("/api/stream", { withCredentials: true })`
- **THEN** recibe `snapshot`, `container-event`, `update-progress` y `notification` en esa única conexión
- **AND** no se abre ninguna conexión SSE adicional para el estado

#### Scenario: La autenticación por cookie funciona en el stream

- **GIVEN** una sesión válida en cookie
- **WHEN** el navegador abre el `EventSource`
- **THEN** el middleware no modifica las cabeceras de la respuesta `text/event-stream`
- **AND** no se emite `Set-Cookie` en esa respuesta

#### Scenario: Reconexión sin pérdida de estado

- **GIVEN** un cliente que perdió la conexión
- **WHEN** `EventSource` reconecta
- **THEN** el primer evento recibido es un `snapshot` con el estado completo actual
- **AND** el cliente no necesita eventos intermedios para reconstruir el estado

### Requirement: El evento `snapshot` SHALL entregar el estado completo al conectar

Al establecerse una conexión, el handler MUST leer `cached_containers` y `progress_cache`, construir un `StateResponse` con `compute_summary` y emitirlo como evento `snapshot` antes de cualquier evento incremental.

#### Scenario: Snapshot inicial

- **GIVEN** la caché de contenedores poblada
- **WHEN** un cliente se conecta a `/api/stream`
- **THEN** el primer evento es `snapshot` con `containers`, `summary` y `progress`
- **AND** su contenido equivale al de `GET /api/state`

### Requirement: El stream SHALL emitir `container-event` por cada cambio de estado

El worker de estado MUST emitir un `ContainerEvent` por cada contenedor cuyo estado cambió respecto al ciclo anterior, sin emitir eventos para contenedores sin cambios.

#### Scenario: Cambio de estado de un contenedor

- **GIVEN** un contenedor `nginx` en estado `exited` en el ciclo anterior
- **WHEN** el worker detecta que pasa a `running`
- **THEN** emite `container-event` con `name="nginx"`, `action` correspondiente y `state="running"`

#### Scenario: Sin cambios, sin eventos

- **GIVEN** un contenedor cuyo estado no cambió
- **WHEN** el worker refresca
- **THEN** NO emite `container-event` para ese contenedor

### Requirement: El progreso de batch SHALL emitirse como `update-progress`

Cada actualización de `progress_cache` y su reset MUST emitir un evento `update-progress` con los contadores actuales, además de notificar al long-poll.

#### Scenario: Progreso durante un batch

- **GIVEN** un batch de check/update en curso
- **WHEN** avanza el contador de revisados
- **THEN** se emite `update-progress` con `total`, `checked`, `updated` y `errors` reales

#### Scenario: Fin de batch

- **GIVEN** un batch que termina
- **WHEN** se resetea el progreso
- **THEN** se emite `update-progress` con los contadores finales o el reset
- **AND** el frontend detecta el fin por ese evento, sin heurísticas de `"__batch__"`

### Requirement: Las notificaciones de UI SHALL emitirse como `notification`

Cuando una política con `notify_events` dispara una notificación, el backend MUST emitir un `NotifEvent` por el canal de notificaciones, además de los envíos salientes (Telegram/Matrix/Webhook).

#### Scenario: Notificación de cambio de estado

- **GIVEN** un contenedor con política `notify_events = true`
- **WHEN** su estado cambia y se notifica
- **THEN** se emite `notification` con `container`, `status` y `timestamp`
- **AND** la UI puede mostrarla en un toast

### Requirement: El stream SHALL emitir heartbeat periódico

El stream MUST fusionar los eventos reales con un intervalo que emite el comentario SSE `: keepalive\n\n` cada 5 segundos como máximo, para atravesar proxies sin buffering. MUST NOT depender de `Sse::keep_alive()`.

#### Scenario: Stream inactivo

- **GIVEN** una conexión a `/api/stream` sin eventos reales
- **WHEN** transcurren más de 5 segundos
- **THEN** se recibe un comentario `: keepalive` cada 5 segundos
- **AND** `EventSource` no dispara ningún evento nombrado

#### Scenario: Stream con actividad

- **GIVEN** una conexión con eventos frecuentes (< 5s)
- **WHEN** se emiten eventos reales
- **THEN** el heartbeat no retrasa ni interfiere con los eventos reales

### Requirement: El estado SHALL seguir disponible por long-poll como respaldo

`GET /api/state` MUST permanecer operativo y devolver un `StateResponse` equivalente al `snapshot` del stream, para que el frontend pueda caer a polling si SSE no está disponible o falla.

#### Scenario: Fallback a polling

- **GIVEN** un cliente donde `EventSource` no existe o supera el umbral de errores
- **WHEN** el hook conmuta a polling
- **THEN** usa `GET /api/state` y recibe `StateResponse`
- **AND** el estado mostrado es consistente con el del stream
