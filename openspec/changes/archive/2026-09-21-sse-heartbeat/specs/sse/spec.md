# Spec Delta: SSE heartbeat keepalive

## ADDED Requirements

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