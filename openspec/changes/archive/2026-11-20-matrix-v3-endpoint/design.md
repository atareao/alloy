# Design

## Contexto

`notify_matrix` monta la URL con el prefijo `r0` y concatena el room ID tal cual. Contra
Continuwuity eso devuelve `M_UNKNOWN` ("non-create event for room of unknown version"), porque `r0`
es una versión legacy que el servidor ya no resuelve como la versión de sala actual. El room ID
(`!room:server`) tampoco se codifica, pese a que la spec lo define como parámetro de ruta.

## Decisiones

### D1 — Endpoint `v3` en lugar de `r0`

`r0` está deprecado; la Client-Server API vigente usa `v3`. FlexGet, que funciona contra el mismo
servidor y la misma sala, usa `v3`. Cambio mínimo y alineado con la spec.

### D2 — Room ID percent-encoded con `percent-encoding`

Se codifica el segmento de ruta con `utf8_percent_encode(room, MATRIX_ROOM_PATH)` de la crate
`percent-encoding` (ya en el lock como transitiva de `reqwest`; se declara como dependencia
directa). `MATRIX_ROOM_PATH` parte de `NON_ALPHANUMERIC` y **quita** los *unreserved* de RFC 3986
(`-`, `.`, `_`, `~`), de modo que `!abc:matrix.org` → `%21abc%3Amatrix.org` —igual que FlexGet
(`urllib.parse.quote`, que deja `_.-~` sin escapar). Se descarta `url::Url::path_segments_mut`
porque su *encode set* de segmento deja `!` sin codificar, que es justo lo que los clientes de
referencia codifican.

### D3 — Sin más cambios

Se conservan `PUT`, `Authorization: Bearer`, `Content-Type: application/json`, el cuerpo
`{"msgtype":"m.notice","body":"🐳 {container}: {status}"}` y el `txn_id` (UUID v4). No se toca el
fan-out ni los demás canales.

## Fuera de alcance

- Migrar a `/_matrix/client/v3/...` con `m.room.message` frente a `m.notice` como `msgtype` (se
  mantiene `m.notice`, aceptado por los servidores).
- Reintentos/backoff de Matrix.
