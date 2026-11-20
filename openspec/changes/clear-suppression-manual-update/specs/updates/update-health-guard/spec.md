# updates/update-health-guard Delta

## ADDED Requirements

### Requirement: Limpieza de supresión en intervención manual masiva (D9/D10)

Los flujos de aplicación manual que actúan sobre **varios** contenedores MUST limpiar la supresión
(`terminal_failed` y `rolled_back`) de cada contenedor que aplican, antes de intentar el update, de forma
equivalente a `update_container_h` (`handlers.rs:628-629`). La intervención manual del operador reinicia
la supresión (D9: "hasta intervención manual"; D10: limpieza ante intervención manual).

- `update_all_h` (`POST /api/update-all`) MUST limpiar ambos guards para **cada contenedor iterado**.
- `update_stack_h` (`POST /api/stacks/{project}/update`) MUST limpiar ambos guards para **cada
  contenedor del stack**.
- Los contenedores `failed-stopped` (parados) NO aparecen en el listado de `update_all_h`
  (`docker_list_running`, `all: false`); MUST documentarse que su reintento se hace vía
  `update_container_h`.

#### Scenario: Contenedor con rollback se reintenta desde update-all

- **Given** un contenedor `db` presente en `RolledBackGuard` (corriendo la imagen antigua tras rollback)
- **And** está en ejecución, por lo que `update_all_h` lo itera
- **When** se invoca `POST /api/update-all`
- **Then** la supresión de rollback de `db` se limpia antes de aplicar
- **And** `db` puede reintentarse (no queda bloqueado por D10)

#### Scenario: Contenedor suprimido se limpia en update-all

- **Given** un contenedor `app` con supresión en `TerminalFailed` que `update_all_h` itera
- **When** se invoca `POST /api/update-all`
- **Then** la entrada de `app` en `TerminalFailed` se limpia

#### Scenario: Contenedor con rollback se reintenta desde update-stack

- **Given** un contenedor `db` de un stack con supresión en `RolledBackGuard`
- **When** se invoca `POST /api/stacks/{project}/update`
- **Then** la supresión de rollback de `db` se limpia antes de aplicar

#### Scenario: failed-stopped requiere update individual

- **Given** un contenedor `app` en estado terminal `failed-stopped` (parado)
- **When** el operador lanza `POST /api/update-all`
- **Then** `app` NO se ve afectado por ese flujo (está parado, no aparece en el listado de running)
- **And** su reintento se realiza vía `POST /api/update/{name}` (update individual)
