# auth Specification

## Purpose
TBD - created by archiving change ws-auth-fix. Update Purpose after archive.

## Requirements

### Requirement: El middleware de auth SHALL no modificar respuestas 101 Switching Protocols

El middleware de autenticación NO DEBE modificar los headers de una respuesta
con status `101 Switching Protocols` (WebSocket upgrade), para no corromper
el handshake de upgrade.

#### Scenario: Petición WebSocket autenticada

- **GIVEN** una petición a `/api/ws?token=<válido>` con headers de upgrade
  WebSocket (`Upgrade: websocket`, `Connection: Upgrade`, `Sec-WebSocket-*`)
- **WHEN** el middleware de auth procesa la respuesta 101 del handler
- **THEN** retorna la respuesta sin modificar headers (sin `Set-Cookie`)
- **AND** el navegador completa el upgrade WebSocket correctamente

#### Scenario: Petición REST normal con sliding session

- **GIVEN** una petición a `/api/containers` con cookie de sesión válida
- **WHEN** el middleware detecta que debe refrescar la sesión
- **THEN** añade `Set-Cookie` a la respuesta (comportamiento actual, sin cambios)
