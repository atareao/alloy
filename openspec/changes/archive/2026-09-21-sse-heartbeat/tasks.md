# Tasks: sse-heartbeat

## TDD Checklist

- [x] RED: test `sse_events_heartbeat_emits_keepalive` — suscribirse al stream
      de `/api/events` (o al stream interno), esperar 6s sin enviar eventos
      reales, verificar que se recibió al menos un `: keepalive`.
- [x] RED: test `sse_heartbeat_does_not_interfere_with_real_events` — enviar
      eventos reales frecuentes, verificar que se entregan sin demora y que
      el heartbeat no los suprime ni retrasa.
- [x] GREEN: implementar heartbeat en los 4 handlers de `events.rs` usando
      `futures::stream::select` + `tokio::time::interval`.
- [x] `cargo test` — verde.
- [x] `cargo check` — sin errores.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` — cero warnings
      (los 2 warnings en `updates/common.rs` son preexistentes, no relacionados).
- [x] `cargo fmt`.
- [ ] `openspec archive sse-heartbeat --yes`