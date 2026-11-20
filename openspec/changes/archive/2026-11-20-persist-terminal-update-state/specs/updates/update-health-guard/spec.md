# updates/update-health-guard Delta

## ADDED Requirements

### Requirement: Restauración de guards desde el historial al arrancar

Al construir el `AppState` (startup), Alloy MUST reconstruir los guards de la guarda de arranque a
partir del historial persistido en `update_history`, de forma que un reinicio del proceso NO pierda la
supresión terminal ni la memoria de rollback. La reconstrucción MUST usar el **último registro por
contenedor** y NO depender del truncado de 100 filas de `load_update_history`.

- `TerminalFailed` MUST contener los contenedores cuyo último registro tenga `status = "failed-stopped"`.
- `RolledBackGuard` MUST contener, por contenedor, el digest intentado = `new_digest` del último
  registro con `status = "rolled-back"`.

#### Scenario: Contenedor failed-stopped sigue suprimido tras reiniciar

- **Given** un historial donde la última entrada del contenedor `app` tiene `status = "failed-stopped"`
- **When** Alloy arranca y construye el `AppState`
- **Then** `app` está presente en `TerminalFailed`
- **And** el scheduler NO propone un nuevo update para `app` (supresión, D9/D10)

#### Scenario: Contenedor rolled-back conserva su digest tras reiniciar

- **Given** un historial donde la última entrada del contenedor `db` tiene `status = "rolled-back"`
  y `new_digest = "sha256:abc"`
- **When** Alloy arranca y construye el `AppState`
- **Then** `RolledBackGuard` mapea `db -> "sha256:abc"`
- **And** `is_rolled_back_suppressed("db", "sha256:abc", map)` es verdadero

#### Scenario: Registro posterior anula la supresión (último gana)

- **Given** un historial con una fila antigua `status = "failed-stopped"` para `app` y una fila
  **posterior** con `status = "updated"`
- **When** Alloy arranca y reconstruye los guards
- **Then** `app` NO está en `TerminalFailed`

#### Scenario: Arranque sin historial de fallos no suprime nada

- **Given** un historial vacío o sin filas `failed-stopped` / `rolled-back`
- **When** Alloy arranca
- **Then** `TerminalFailed` y `RolledBackGuard` quedan vacíos
- **And** el comportamiento es idéntico al actual

#### Scenario: Replay resistente al truncado del historial

- **Given** un contenedor cuyo estado terminal NO está entre las 100 filas más recientes de
  `load_update_history` pero su último registro sí es terminal
- **When** Alloy arranca
- **Then** el guard correspondiente se reconstruye correctamente (el replay usa una consulta del
  último estado por contenedor, no la lista truncada)

### Requirement: Poda por presencia y reconciliación en el primer ciclo

La restauración MUST descartar entradas de contenedores que ya no existen en el daemon (poda por
presencia; el runtime poda con `all: true` porque un `failed-stopped` está **parado pero existe**).
La restauración NO MUST depender de la tabla `containers`: dicha tabla **no se rellena en producción**
(`save_containers` es código muerto y los `UPDATE containers ...` existentes son no-ops), por lo que
`has_update` no puede usarse como fuente de elegibilidad.

Una supresión restaurada que ya NO corresponda a un update pendiente MUST eliminarla el scheduler en su
**primer ciclo**, reutilizando la limpieza de runtime existente (`scheduler.rs:214-223`, condición
`!decision.has_update`, que usa el estado **en vivo**). Así la persistencia no puede dejar una supresión
obsoleta indefinidamente: el estado converge al real en el primer ciclo.

#### Scenario: Contenedor suprimido que ya no existe se descarta

- **Given** un historial con `status = "failed-stopped"` para `ghost`, y `ghost` NO está presente en el daemon
- **When** Alloy arranca y reconstruye los guards
- **Then** `ghost` NO está en `TerminalFailed`

#### Scenario: Supresión obsoleta se reconcilia en el primer ciclo

- **Given** un contenedor `app` restaurado en `TerminalFailed` desde el historial
- **And** su decisión en vivo tiene `has_update = false` (no hay update pendiente)
- **When** el scheduler ejecuta su primer ciclo
- **Then** la limpieza de runtime elimina `app` de `TerminalFailed`
- **And** `app` deja de estar suprimido (no queda suprimido indefinidamente)

#### Scenario: Supresión vigente se mantiene (D9)

- **Given** un contenedor `app` restaurado en `TerminalFailed` desde el historial
- **And** su decisión en vivo tiene `has_update = true` (sigue habiendo update pendiente)
- **When** el scheduler ejecuta su primer ciclo
- **Then** `app` sigue suprimido (D9) hasta intervención manual

#### Scenario: Error al cargar el historial no deja la feature muda

- **Given** un fallo al consultar el historial en el bootstrap
- **When** Alloy arranca
- **Then** los guards quedan vacíos (fail-open) **y** se emite un `tracing::warn!` con el error
  (la persistencia no se aplica silenciosamente)

### Requirement: Constantes centralizadas de estado terminal

Los strings de estado terminal `"rolled-back"` y `"failed-stopped"` MUST definirse **una sola vez** y
reutilizarse en todos los puntos de escritura/lectura (scheduler, guard, replay y notificaciones),
eliminando las repeticiones actuales (`scheduler.rs:368,395,525,552`, `health_guard.rs:137-138`,
`notifications.rs:265,267`).

#### Scenario: Un único origen para los estados terminales

- **Given** el módulo de la guarda de arranque
- **When** el scheduler, la guarda o las notificaciones escriben o comparan un estado terminal
- **Then** usan las constantes centralizadas, no literales duplicados
