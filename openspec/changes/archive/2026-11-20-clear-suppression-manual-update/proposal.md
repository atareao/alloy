# Proposal

## Why

El diseño de la guarda de arranque (feature #125) exige limpiar la supresión ante **intervención
manual**:

- **D10** (`archive/2026-11-20-update-rollback-guard/design.md:149`): *«Limpieza ante intervención
  manual (`update_container_h`/`update_all_h`) o `has_update=false`»*.
- **D9** (`.../design.md:134-135`): limpieza en el update/start manual del contenedor.

Hoy sólo `update_container_h` lo cumple (`backend/src/updates/handlers.rs:628-629`). `update_all_h`
(`POST /api/update-all`, `handlers.rs:855`) y `update_stack_h` (`POST /api/stacks/{project}/update`,
`stacks.rs:168`) **escriben** supresiones (vía `record_guard_outcome`) pero **nunca las limpian**.

**Impacto real:** un contenedor en `rolled_back` sigue corriendo la imagen antigua, así que aparece en
`docker_list_running` (`all: false`, `workers/state.rs:15`) y `update_all_h` lo itera; al no limpiar su
supresión de rollback, el reintento queda bloqueado (contradice D10).

## What Changes

- `update_all_h`: limpiar `TerminalFailed` y `RolledBackGuard` para **cada contenedor iterado**, antes
  de aplicar (espejo exacto de `update_container_h:628-629`).
- `update_stack_h`: mismo tratamiento para los contenedores del stack.
- Documentar que los contenedores `failed-stopped` (parados) **no** entran en el listado de
  `update_all_h` (`all: false`), por lo que se reintentan vía `update_container_h`.

## Capabilities

### Modified Capabilities

- `updates/update-health-guard`: se explicita el requisito de **limpieza de supresión en intervención
  manual masiva** (D9/D10) en los flujos `update_all_h` y `update_stack_h`.

### New Capabilities

- Ninguna.

## Impact

- **Backend**: `updates/handlers.rs` (`update_all_h`), `stacks.rs` (`update_stack_h`). Reutiliza los
  newtypes `State<TerminalFailed>` / `State<RolledBackGuard>` ya inyectados; sin API nueva.
- **API**: sin cambios de contrato (mismos endpoints).
- **Frontend**: sin cambios.
- **Datos**: ninguno.
- **Dependencias**: ninguna.
