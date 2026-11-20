# notifications Specification

## ADDED Requirements

### Requirement: Notificador Matrix

Alloy MUST enviar los cambios de estado de contenedor a Matrix mediante
`PUT {homeserver}/_matrix/client/v3/rooms/{room_id}/send/m.room.message/{txn_id}`, autenticado con
`Authorization: Bearer {matrix_token}` y `Content-Type: application/json`. El `homeserver` MUST
normalizarse eliminando las barras finales. El `room_id` MUST percent-encodearse para el segmento de
ruta. El `txn_id` MUST ser único por envío. El cuerpo MUST ser
`{"msgtype":"m.notice","body":"🐳 {container}: {status}"}`. El canal MUST considerarse configurado
solo cuando `matrix_homeserver`, `matrix_token` y `matrix_room` estén presentes; si falta
cualquiera, `notify_matrix` MUST devolver `Err` sin emitir petición HTTP. MUST NOT usar el prefijo
legacy `r0`.

#### Scenario: Envío correcto con el endpoint v3

- **Given** `matrix_homeserver=https://hs.test`, `matrix_token` y `matrix_room=!room:hs.test`
- **When** se ejecuta `notify_matrix` para el contenedor `app` con estado `running`
- **Then** se hace `PUT` a
  `https://hs.test/_matrix/client/v3/rooms/%21room%3Ahs.test/send/m.room.message/{txn_id}`
- **And** la petición lleva `Authorization: Bearer ...` y `Content-Type: application/json`
- **And** el cuerpo es `{"msgtype":"m.notice","body":"🐳 app: running"}`
- **And** `notify_matrix` devuelve `Ok(())` y registra un `INFO`

#### Scenario: El ID de sala va codificado y sin `r0`

- **Given** `matrix_room=!abc:matrix.org`
- **When** se ejecuta `notify_matrix`
- **Then** la ruta contiene `/v3/rooms/%21abc%3Amatrix.org/`
- **And** la ruta NO contiene `/r0/` ni el ID de sala sin codificar

#### Scenario: Homeserver con barras finales

- **Given** `matrix_homeserver=https://hs.test/`
- **When** se ejecuta `notify_matrix`
- **Then** la URL no contiene dobles barras antes de `_matrix`

#### Scenario: Matrix no configurado

- **Given** que falta `matrix_homeserver`, `matrix_token` o `matrix_room`
- **When** se ejecuta `notify_matrix`
- **Then** devuelve `Err` con un mensaje "Matrix no configurado"
- **And** NO se emite ninguna petición HTTP

#### Scenario: Matrix rechaza la notificación

- **Given** Matrix configurado y un servidor que responde `500` con `{"errcode":"M_UNKNOWN",...}`
- **When** se ejecuta `notify_matrix`
- **Then** devuelve `Err` con el código HTTP y el cuerpo de la respuesta
- **And** se registra un `ERROR`

#### Scenario: Error de conexión con Matrix

- **Given** que `matrix_homeserver` no es alcanzable
- **When** se ejecuta `notify_matrix`
- **Then** devuelve `Err` con la causa de conexión
- **And** se registra un `ERROR`
