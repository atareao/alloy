# Tasks: sse-nodelay-fix

## TDD Checklist

- [ ] RED: Escribir test de integración (o unit test aislado) que verifique que
      `ListenerExt::tap_io` se usa envolviendo el `TcpListener`, y que una conexión
      real aceptada tiene `TCP_NODELAY` activo (verificable vía `TcpStream::nodelay()`
      en un test que levanta el listener modificado y acepta una conexión de loopback).
- [ ] GREEN: Modificar `backend/src/main.rs`:
      - Reemplazar el flujo `TcpSocket -> set_nodelay -> bind -> listen` (que no
        propaga nodelay) por: `TcpSocket -> bind -> listen -> into_std -> TcpListener::from_std`
        (o directamente `tokio::net::TcpListener::bind(addr)`) `.tap_io(|s| { let _ = s.set_nodelay(true); })`.
      - Pasar el listener resultante a `axum::serve(...)`.
- [ ] Ejecutar `cargo test` — confirmar verde.
- [ ] Ejecutar `cargo check` y `cargo clippy -- -D warnings` — confirmar cero warnings.
- [ ] REFACTOR: `cargo fmt`.
- [ ] Verificación manual (opcional): `curl -N http://localhost:PORT/api/stream` mientras
      se generan eventos, confirmar entrega evento-por-evento sin ráfagas perceptibles.
- [ ] Archivar: `openspec archive sse-nodelay-fix --yes`.
