# Spec Delta: WebSocket endpoint

## ADDED Requirements

### Requirement: El sistema SHALL exponer un endpoint WebSocket `/api/ws`

El sistema DEBE exponer `GET /api/ws` que realice upgrade WebSocket
(`axum::extract::ws::WebSocketUpgrade`). La autenticación se realiza vía
`?token=` en query string (mecanismo ya existente en `auth_middleware`).

**Contrato:**

```
GET /api/ws?token=<jwt-de-sesión>
Upgrade: websocket
Connection: upgrade
Sec-WebSocket-Key: <key>
Sec-WebSocket-Version: 13

→ 101 Switching Protocols
```

#### Scenario: Usuario autenticado conecta vía WebSocket

- **GIVEN** un usuario con sesión válida (token JWT no expirado)
- **WHEN** abre `new WebSocket("wss://.../api/ws?token=...")`
- **THEN** el servidor responde con upgrade 101
- **AND** comienza a recibir mensajes JSON con eventos en tiempo real

#### Scenario: Usuario no autenticado intenta conectar

- **GIVEN** una petición sin token o con token inválido/expirado
- **WHEN** intenta conectar a `/api/ws`
- **THEN** el servidor responde 401 (rechaza el upgrade)
- **AND** la conexión WebSocket no se establece

### Requirement: Los mensajes WebSocket SHALL tener formato JSON con campo `type`

Cada mensaje enviado por el servidor DEBE ser un objeto JSON con un campo
`type` que identifica el tipo de evento, y un campo `data` con el payload.

**Formatos:**

```json
{"type": "containers", "data": {"containers": [...]}}
{"type": "update-progress", "data": {"container": "nginx", "status": "Pulling", ...}}
{"type": "notification", "data": {"container": "nginx", "status": "running → exited", ...}}
```

#### Scenario: Se emite un evento de estado de contenedores

- **GIVEN** el `state_worker` emite un `StateEvent` al broadcast channel
- **WHEN** el handler WebSocket recibe el evento
- **THEN** envía un mensaje `{"type": "containers", "data": {...}}` al cliente

#### Scenario: Se emite un evento de progreso de update

- **GIVEN** el `update_check_worker` emite un `UpdateProgress` al broadcast
- **WHEN** el handler WebSocket recibe el evento
- **THEN** envía un mensaje `{"type": "update-progress", "data": {...}}`

#### Scenario: Se emite una notificación

- **GIVEN** el `state_worker` emite un `NotifEvent` al broadcast
- **WHEN** el handler WebSocket recibe el evento
- **THEN** envía un mensaje `{"type": "notification", "data": {...}}`

### Requirement: El handler WebSocket SHALL mantener la conexión activa

El handler DEBE mantener la conexión WebSocket abierta mientras el cliente
esté conectado, escuchando los 3 broadcast channels simultáneamente
(`tokio::select!` o `futures::stream::select_all`). Si el broadcast channel
tiene error `Lagged`, debe continuar (no cerrar la conexión).

#### Scenario: Cliente se desconecta

- **GIVEN** una conexión WebSocket activa
- **WHEN** el cliente cierra la conexión o se desconecta
- **THEN** el handler limpia los recursos y termina gracefulmente

#### Scenario: Broadcast channel tiene Lagged

- **GIVEN** un cliente lento que no alcanza a leer todos los eventos
- **WHEN** el broadcast channel descarta eventos por `Lagged`
- **THEN** el handler continúa enviando eventos nuevos sin cerrar la conexión