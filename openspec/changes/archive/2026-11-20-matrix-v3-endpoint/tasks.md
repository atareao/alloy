# Tasks

## 1. RED — Reproducir la URL incorrecta

- [x] 1.1 Test que captura la petición con un `tokio::net::TcpListener` local y afirma que la ruta
      es `/_matrix/client/v3/rooms/<room-encoded>/send/m.room.message/<txn>`.
- [x] 1.2 Test que afirma que la ruta NO contiene `/r0/` y que el room ID va percent-encoded.
- [x] 1.3 Tests de early-return (Matrix no configurado) y de error (rechazo / conexión).
- [x] 1.4 Tests de los escenarios restantes del spec: barras finales, `txn_id` único,
      HTTP 500 con cuerpo y error de conexión; y aserción de `Content-Type`.

## 2. GREEN — Endpoint v3 + room codificado

- [x] 2.1 `notify_matrix`: `v3` + `utf8_percent_encode(room, MATRIX_ROOM_PATH)` (deja `-._~` sin codificar).
- [x] 2.2 `Cargo.toml`: añadir `percent-encoding = "2"`.

## 3. REFACTOR y verificación

- [x] 3.1 `cargo fmt -- --check` y `cargo clippy --locked -- -D warnings`.
- [x] 3.2 `cargo test` en verde.

## 4. Documentación y cierre

- [ ] 4.1 Auditar, abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
