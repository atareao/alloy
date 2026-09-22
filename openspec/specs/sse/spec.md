# sse Specification

## Purpose
TBD - created by archiving change sse-nodelay-fix. Update Purpose after archive.

## Requirements

### Requirement: El servidor SHALL aplicar TCP_NODELAY a cada conexión TCP aceptada

El servidor HTTP DEBE deshabilitar el algoritmo de Nagle (`TCP_NODELAY = true`) en
cada socket de conexión individual aceptado por el listener, no solo en el socket
de escucha, para garantizar que los chunks de streaming (SSE) se envíen sin
retraso de agrupación del kernel.

#### Scenario: Nueva conexión de cliente SSE

- **GIVEN** el servidor Alloy está escuchando en el puerto configurado
- **WHEN** un cliente abre una conexión (p.ej. `GET /api/events`, `GET /api/stream`)
- **THEN** el `TcpStream` aceptado para esa conexión tiene `TCP_NODELAY` habilitado
- **AND** los eventos emitidos por el `broadcast::Sender` correspondiente se escriben
  en el socket sin esperar a que el buffer de Nagle se llene o expire el timer

#### Scenario: Fallo al establecer TCP_NODELAY en una conexión

- **GIVEN** una conexión aceptada donde `set_nodelay(true)` devuelve `Err`
  (caso raro, p.ej. socket ya cerrado)
- **WHEN** se intenta aplicar la opción
- **THEN** el servidor registra un `tracing::trace!`/`warn!` con el error
- **AND** continúa sirviendo la conexión con el comportamiento por defecto
  (no debe abortar ni hacer panic)

### Requirement: El código de arranque NO SHALL depender de nodelay en el socket de escucha

El código NO DEBE asumir que `set_nodelay` en el socket de escucha (`TcpSocket`)
se propaga a las conexiones aceptadas, ya que esto es falso en Linux/BSD.

#### Scenario: Revisión de código

- **GIVEN** el bloque de arranque en `main.rs`
- **WHEN** se revisa el manejo de `TCP_NODELAY`
- **THEN** la única llamada relevante a `set_nodelay(true)` que afecta el
  comportamiento de streaming está en el `tap_io` aplicado sobre el listener
  ya convertido (`TcpListener` de tokio), no sobre el `TcpSocket` previo al bind

### Requirement: Los endpoints SSE SHALL emitir un heartbeat periódico

Cada uno de los 4 endpoints SSE (`/api/events`, `/api/updates`,
`/api/notifications`, `/api/stream`) DEBE fusionar el stream de eventos
reales con un stream de intervalo que emite un comentario SSE
(`: keepalive\n\n`) cada 5 segundos. Esto fuerza el envío de frames
HTTP/2 a través de proxies (Traefik) que de otro modo acumularían eventos
pequeños hasta llenar el tamaño de frame.

#### Scenario: Conexión SSE inactiva sin eventos reales

- **GIVEN** un cliente conectado a `/api/stream` (o cualquier endpoint SSE)
- **WHEN** no hay eventos reales emitidos durante más de 5 segundos
- **THEN** el servidor envía `: keepalive\n\n` cada 5 segundos
- **AND** el `EventSource` del cliente ignora el comentario (no dispara
  ningún evento, no afecta al estado de la conexión)

#### Scenario: Conexión SSE con eventos reales frecuentes

- **GIVEN** un cliente conectado a un endpoint SSE
- **WHEN** hay eventos reales emitiéndose con frecuencia (< 5s entre ellos)
- **THEN** el heartbeat no interfiere ni retrasa los eventos reales
- **AND** los eventos reales se entregan inmediatamente, sin esperar al
  heartbeat

#### Scenario: Heartbeat no rompe la semántica SSE

- **GIVEN** un cliente conectado a un endpoint SSE
- **WHEN** recibe el heartbeat `: keepalive\n\n`
- **THEN** el `EventSource` no dispara ningún evento nombrado
- **AND** la conexión permanece abierta y funcional para eventos posteriores

### Requirement: El heartbeat SHALL implementarse fusionando streams

La implementación DEBE usar `futures::stream::select` o
`stream::select_all` para fusionar el stream de eventos reales con un
stream generado por `tokio::time::interval`. No debe usarse
`Sse::keep_alive()` de axum porque esa función solo envía un comentario
cuando el stream está *bloqueado* (no produce items), no garantiza el
envío periódico independientemente de la actividad del stream.

#### Scenario: Verificación de implementación

- **GIVEN** el código del handler SSE en `events.rs`
- **WHEN** se inspecciona la construcción del stream
- **THEN** el stream combina eventos reales con un stream de intervalo
  usando `select`/`select_all`
- **AND** no se usa `Sse::keep_alive()` como sustituto del heartbeat
