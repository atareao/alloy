# Tasks

## 1. RED — Test que falla

- [x] 1.1 `notifications.rs`: test de que durante `notify_all_state` el lock de `Settings`
      queda libre (p. ej. un canal de prueba que intenta `try_lock` o que otro `lock().await`
      progresa mientras un canal duerme).
- [x] 1.2 `cargo test` en rojo.

## 2. GREEN — Implementación mínima

- [x] 2.1 `notifications.rs::notify_all_state`: clonar `Settings` bajo el lock, soltar el
      guard y lanzar el `tokio::join!` sobre el clon.
- [x] 2.2 `cargo test` verde + `cargo check`.

## 3. REFACTOR — Verificación

- [x] 3.1 `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.2 `cargo test` sin regresiones.
- [x] 3.3 Revisión con `rust-reviewer`.

## 4. Archive

- [x] 4.1 `openspec validate notify-fanout-lock --strict`
- [x] 4.2 `openspec archive notify-fanout-lock --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/notify-fanout-lock/`
