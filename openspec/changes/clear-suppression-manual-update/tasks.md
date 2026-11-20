# Tasks — clear-suppression-manual-update

## 1. RED (tests que fallan)

- [ ] 1.1 Backend: test de que `update_all_h` limpia `RolledBackGuard` del contenedor iterado.
- [ ] 1.2 Backend: test de que `update_all_h` limpia `TerminalFailed` del contenedor iterado.
- [ ] 1.3 Backend: test de que `update_stack_h` limpia ambos guards para los contenedores del stack.
- [ ] 1.4 Backend: test de que `update_container_h` mantiene su limpieza (no regresión).

## 2. GREEN (implementación mínima)

- [ ] 2.1 `updates/handlers.rs` `update_all_h`: limpiar `terminal_failed`/`rolled_back` por contenedor
      antes de aplicar (espejo de `update_container_h:628-629`).
- [ ] 2.2 `stacks.rs` `update_stack_h`: misma limpieza por contenedor del stack.
- [ ] 2.3 Documentar en código que `failed-stopped` (parado) no entra en `update_all_h`.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` en verde.
- [ ] 3.2 `cd backend && cargo test` en verde.
- [ ] 3.3 Sin cambios de contrato API/frontend.
- [ ] 3.4 Docs: `docs/clear-suppression-manual-update.md`.

## 4. Archive

- [ ] 4.1 `openspec archive clear-suppression-manual-update` tras merge.
