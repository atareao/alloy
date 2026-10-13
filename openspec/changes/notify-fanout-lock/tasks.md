# Tasks

## 1. RED — Test que falla

- [ ] 1.1 `notifications.rs`: test de que durante `notify_all_state` el lock de `Settings`
      queda libre (p. ej. un canal de prueba que intenta `try_lock` o que otro `lock().await`
      progresa mientras un canal duerme).
- [ ] 1.2 `cargo test` en rojo.

## 2. GREEN — Implementación mínima

- [ ] 2.1 `notifications.rs::notify_all_state`: clonar `Settings` bajo el lock, soltar el
      guard y lanzar el `tokio::join!` sobre el clon.
- [ ] 2.2 `cargo test` verde + `cargo check`.

## 3. REFACTOR — Verificación

- [ ] 3.1 `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] 3.2 `cargo test` sin regresiones.
- [ ] 3.3 Revisión con `rust-reviewer`.

## 4. Archive

- [ ] 4.1 `openspec validate notify-fanout-lock --strict`
- [ ] 4.2 `openspec archive notify-fanout-lock --yes`
- [ ] 4.3 Verificar/eliminar `openspec/changes/notify-fanout-lock/`
