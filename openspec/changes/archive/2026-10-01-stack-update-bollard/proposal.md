# Proposal

## Why

El change `update-check-robustness` corrigió el no-op silencioso de `PullRestartStack` en
el worker del scheduler migrándolo a la API Bollard. Sin embargo, el mismo defecto de raíz
persiste en el resto de caminos de stack: todos invocan el CLI `docker compose`, pero la
imagen runtime (`Dockerfile:50-52`) solo instala `ca-certificates`, **no el CLI de Docker**.
En consecuencia, en la imagen publicada fallan:

- `updates/handlers.rs:1327,1337` — `apply_single_policy` (rama `PullRestartStack`, ruta UI
  "check all" / aplicar políticas): feature rota (loguea `ERROR`, pero nunca actualiza).
- `stacks.rs:22` — `get_compose_projects` (`docker compose ls`): devuelve `HashMap` vacío,
  rompiendo el fallback de resolución de compose file.
- `stacks.rs:160,166` — `update_stack_h` (`POST /api/stacks/{project}/update`): devuelve
  `status:"error"` siempre.
- `stacks.rs:326` — `down_stack_h` (`POST /api/stacks/{project}/down`): falla siempre.

## What Changes

- Migrar `update_stack_h` y la rama `PullRestartStack` de `apply_single_policy` a la API
  Bollard, reutilizando los helpers ya existentes (`pull_image`, `recreate_container`).
- Migrar `down_stack_h` a la API (`stop_container` + `remove_container` por contenedor del
  proyecto), sin CLI.
- Eliminar el descubrimiento CLI (`get_compose_projects`, `get_compose_project_path`) y el
  fallback de compose file; el descubrimiento por labels ya es el camino principal.
- Retirar `resolve_compose_file` (queda muerto) y su re-export en `workers/mod.rs`.
- Errores visibles en todos los caminos (`StackUpdateResult.status="error"` con causa).

## Capabilities

### New Capabilities
- `stacks/lifecycle`: actualización y bajada de stacks Docker Compose vía API de Docker
  (Bollard), sin dependencia del CLI `docker`.

### Modified Capabilities
<!-- Ninguna. -->

## Impact

- `backend/src/stacks.rs`: `update_stack_h`, `down_stack_h`, eliminación de
  `get_compose_projects`/`get_compose_project_path`, tests asociados.
- `backend/src/updates/handlers.rs`: rama `PullRestartStack` de `apply_single_policy` y el
  import de `resolve_compose_file`.
- `backend/src/workers/mod.rs`: quitar el re-export de `resolve_compose_file`.
- `backend/src/workers/scheduler.rs`: eliminar `resolve_compose_file` (sin llamadas tras el
  cambio); verificar con grep.
- No se toca el `Dockerfile`. Sin cambios de frontend ni de schema SQLite.

Fuera de alcance: semántica completa de `docker compose down` (volúmenes/nets con nombre) y
`depends_on`; ver Open Questions.
