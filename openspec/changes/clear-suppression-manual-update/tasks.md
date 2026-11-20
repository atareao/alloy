# Tasks — clear-suppression-manual-update

## 1. RED (tests que fallan)

- [x] 1.1 Backend: test de que `update_all_h` limpia `RolledBackGuard` del contenedor iterado.
- [x] 1.2 Backend: test de que `update_all_h` limpia `TerminalFailed` del contenedor iterado.
- [x] 1.3 Backend: test de que `update_stack_h` limpia ambos guards para los contenedores del stack.
- [x] 1.4 Backend: test de que `update_container_h` mantiene su limpieza (no regresión).

> **Decisión de nivel de test**: los handlers completos (`update_all_h`/`update_stack_h`)
> requieren un daemon de contenedores (`docker_list_running` / `docker.list_containers`), que
> el repo sólo ejercita en tests `#[ignore]` con `test_support::podman_client` y no tiene
> patrón de mock de `Docker` inyectable. Por eso 1.1–1.4 se cubren a nivel de helper
> `clear_suppression` (que es exactamente el punto único de limpieza usado por los tres
> flujos) + un test de contrato de bucle (`clear_suppression_clears_every_iterated_container`)
> que espeja la iteración por contenedor.

## 2. GREEN (implementación mínima)

- [x] 2.1 `updates/handlers.rs` `update_all_h`: limpiar `terminal_failed`/`rolled_back` por contenedor
      antes de aplicar (espejo de `update_container_h:628-629`).
- [x] 2.2 `stacks.rs` `update_stack_h`: misma limpieza por contenedor del stack.
- [x] 2.3 Documentar en código que `failed-stopped` (parado) no entra en `update_all_h`.

## 3. REFACTOR / verificación

- [x] 3.1 `cd backend && just check` en verde.
- [x] 3.2 `cd backend && cargo test` en verde.
- [x] 3.3 Sin cambios de contrato API/frontend.
- [x] 3.4 Docs: `docs/clear-suppression-manual-update.md`.

## 4. Archive

- [ ] 4.1 `openspec archive clear-suppression-manual-update` tras merge.
