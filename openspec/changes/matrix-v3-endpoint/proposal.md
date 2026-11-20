# Proposal

## Why

Las notificaciones de Matrix de Alloy fallan contra servidores modernos (p. ej. **Continuwuity**).
Alloy usa el endpoint legacy `/_matrix/client/r0/rooms/{room}/send/m.room.message/{txn}` y el
servidor responde:

```
HTTP 500 {"errcode":"M_UNKNOWN","error":"non-create event for room of unknown version in !ROOM:..."}
```

El endpoint vigente de la Client-Server API es **`v3`**. Además, el ID de sala (`!room:server`) se
inserta en la ruta **sin codificar**; la spec de Matrix lo trata como segmento de ruta y los
clientes de referencia lo percent-encodean — FlexGet, que sí funciona contra el mismo servidor,
usa `v3` **y** codifica la sala (issue #123).

## What Changes

- `backend/src/notifications.rs`: `notify_matrix` pasa a
  `PUT {homeserver}/_matrix/client/v3/rooms/{room_id_encoded}/send/m.room.message/{txn_id}`.
- El `room_id` MUST percent-encodearse para el segmento de ruta (`!abc:matrix.org` →
  `%21abc%3Amatrix.org`).
- `backend/Cargo.toml`: dependencia directa `percent-encoding = "2"` (ya presente en `Cargo.lock`
  como transitiva; no añade crates nuevos).
- Sin cambios en el cuerpo (`m.notice`), la autenticación (`Bearer`) ni el resto de canales.

## Capabilities

### Added Capabilities

- `notifications`: nuevo requisito **"Notificador Matrix"** (endpoint `v3` + room ID codificado),
  hoy no especificado.

## Impact

- `backend/src/notifications.rs`, `backend/Cargo.toml`, spec `notifications`.
- Solo backend; sin cambios de API, de UI ni del fan-out.
