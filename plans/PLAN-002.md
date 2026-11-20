# PLAN-002 — Limpiar supresión terminal/rollback en intervención manual masiva

- **Issue**: #132 (https://github.com/atareao/alloy/issues/132)
- **Change OpenSpec**: `clear-suppression-manual-update`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/clear-suppression-manual-update` desde `develop`; PR a `develop`.

## Objetivo

Cumplir D9/D10 de la guarda de arranque: los flujos de aplicación manual masiva (`update_all_h`,
`update_stack_h`) deben limpiar la supresión (`terminal_failed` y `rolled_back`) de cada contenedor que
aplican, igual que ya hace `update_container_h`.

## Problema

- D10 (`archive/2026-11-20-update-rollback-guard/design.md:149`) exige limpieza en intervención manual
  (`update_container_h`/`update_all_h`); D9 (`:134-135`) para el update manual.
- `update_container_h` SÍ limpia (`backend/src/updates/handlers.rs:628-629`).
- `update_all_h` (`POST /api/update-all`, `handlers.rs:855`) NO limpia (solo escribe vía `record_guard_outcome`, `:1039`).
- `update_stack_h` (`POST /api/stacks/{project}/update`, `stacks.rs:168`) comparte el gap.
- Caso real: un contenedor en `rolled_back` corre la imagen antigua → aparece en `docker_list_running`
  (`all:false`, `workers/state.rs:15`) → `update_all_h` lo itera pero no limpia su supresión → reintento bloqueado.

## Alcance

1. `update_all_h`: limpiar ambos guards por contenedor iterado, antes de aplicar.
2. `update_stack_h`: ídem por contenedor del stack.
3. Documentar que `failed-stopped` (parado) no entra en `update_all_h`; se reintenta vía `update_container_h`.

## Fuera de alcance

- `check_all_h` / `apply_single_policy` (automático por políticas): el design solo menciona flujos manuales.

## Ficheros implicados

- `backend/src/updates/handlers.rs` (`update_all_h`).
- `backend/src/stacks.rs` (`update_stack_h`).

## Plan TDD

### RED
- Tests de que `update_all_h` limpia `RolledBackGuard`/`TerminalFailed` del contenedor iterado.
- Test de que `update_stack_h` limpia ambos guards por contenedor del stack.
- Test de no regresión de `update_container_h`.

### GREEN
- Replicar el patrón de `update_container_h:628-629` (`terminal_failed.0.lock().await.remove(&name)` /
  `rolled_back.0.lock().await.remove(&name)`) en cada iteración.

### REFACTOR / verificación
- `cd backend && just check` y `cargo test` en verde.
- Sin cambios de contrato API/frontend.

## Verificación de cierre

- [ ] Spec aprobado por el usuario (`.approved` presente).
- [ ] RED → GREEN → REFACTOR completados.
- [ ] `just check` y `cargo test` en verde.
- [ ] `docs/clear-suppression-manual-update.md` creado.
- [ ] `openspec archive clear-suppression-manual-update` + limpieza del directorio.
- [ ] PR a `develop` mergeado e issue #132 cerrado.

## Referencias

- #125 / PR #129; follow-up 1 #130 / PR #131.
- Spec delta: `openspec/changes/clear-suppression-manual-update/specs/updates/update-health-guard/spec.md`.
