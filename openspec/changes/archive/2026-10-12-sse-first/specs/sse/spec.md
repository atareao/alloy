# Spec Delta

## MODIFIED Requirements

### Requirement: Los endpoints SSE SHALL emitir un heartbeat periódico

Cada endpoint SSE vigente (`GET /api/stream` para estado/progreso/notificaciones y los endpoints de logs `/api/containers/{name}/logs` y `/api/stacks/{project}/logs`) MUST garantizar el avance del stream a través de proxies. Para `/api/stream` el heartbeat MUST emitirse fusionando el stream de eventos reales con un stream de intervalo que emite un comentario SSE (`: keepalive\n\n`) cada 5 segundos. Esto fuerza el envío de frames HTTP/2 a través de proxies (Traefik) que de otro modo acumularían eventos pequeños hasta llenar el tamaño de frame.

#### Scenario: Conexión SSE inactiva sin eventos reales

- **GIVEN** un cliente conectado a `/api/stream`
- **WHEN** no hay eventos reales emitidos durante más de 5 segundos
- **THEN** el servidor envía `: keepalive\n\n` cada 5 segundos
- **AND** el `EventSource` del cliente ignora el comentario (no dispara ningún evento nombrado)

#### Scenario: Conexión SSE con eventos reales frecuentes

- **GIVEN** un cliente conectado a `/api/stream`
- **WHEN** hay eventos reales emitiéndose con frecuencia (< 5s entre ellos)
- **THEN** el heartbeat no interfiere ni retrasa los eventos reales
- **AND** los eventos reales se entregan inmediatamente, sin esperar al heartbeat

#### Scenario: Heartbeat no rompe la semántica SSE

- **GIVEN** un cliente conectado a `/api/stream`
- **WHEN** recibe el heartbeat `: keepalive\n\n`
- **THEN** el `EventSource` no dispara ningún evento nombrado
- **AND** la conexión permanece abierta y funcional para eventos posteriores
