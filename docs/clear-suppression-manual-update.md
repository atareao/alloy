# clear-suppression-manual-update

## Qué es

Limpieza de la supresión de updates (`terminal_failed` y `rolled_back`) en los
flujos de **intervención manual masiva**: `POST /api/update-all`
(`update_all_h`) y `POST /api/stacks/{project}/update` (`update_stack_h`).

Cierra el incumplimiento de los design decisions D9/D10 de la guarda de arranque
(`openspec/changes/archive/2026-11-20-update-rollback-guard/design.md`): la
intervención manual del operador MUST reiniciar la supresión. Antes sólo lo hacía
`update_container_h` (`POST /api/update/{name}`).

## Ficheros implicados

- `backend/src/updates/handlers.rs`
  - nuevo helper `pub(crate) async fn clear_suppression(name, &TerminalFailed, &RolledBackGuard)`.
  - `update_container_h`: refactorizado para usar el helper (comportamiento idéntico).
  - `update_all_h`: llamada al helper **por cada contenedor iterado**, al inicio del bucle.
- `backend/src/stacks.rs`
  - `update_stack_h`: llamada al helper **por cada contenedor del stack**, antes de aplicar.
- `backend/src/state.rs` (sin cambios): newtypes `TerminalFailed` / `RolledBackGuard`.

## Cómo funciona

`clear_suppression` es la operación inversa de `record_guard_outcome`:

```rust
terminal_failed.0.lock().await.remove(name);
rolled_back.0.lock().await.remove(name);
```

Al ser el **punto único** de limpieza manual, los tres flujos
(`update_container_h`, `update_all_h`, `update_stack_h`) comparten semántica
exacta y el helper es testeable de forma aislada (sin daemon de contenedores).

## Variables / configuración

Ninguna nueva.

## failed-stopped y update-all

Un contenedor en estado terminal `failed-stopped` está **parado**. `update_all_h`
itera sólo contenedores en ejecución (`docker_list_running`, `all: false`), por lo
que **no lo cubre**. Su reintento se realiza vía `update_container_h`
(`POST /api/update/{name}`). Documentado en el comentario del bucle de
`update_all_h` y en el docstring del helper.

## Decisión de test

Los handlers completos requieren un daemon (`docker_list_running` /
`docker.list_containers`); el repo sólo prueba esa ruta con tests `#[ignore]` vía
`test_support::podman_client` y no dispone de un mock de `Docker` inyectable. La
cobertura se hace:

- a nivel de helper: `clear_suppression_clears_both_guards`,
  `clear_suppression_is_scoped_per_container`, `clear_suppression_is_noop_when_absent`;
- contrato de bucle: `clear_suppression_clears_every_iterated_container` (espeja la
  iteración por contenedor de `update_all_h`/`update_stack_h`).

## Follow-ups

- Ninguno. Sin cambios de contrato API ni frontend.
