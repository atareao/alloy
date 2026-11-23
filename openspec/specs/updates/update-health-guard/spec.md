# updates/update-health-guard Specification

## Purpose
Esta capability especifica la guarda de salud del rollback de updates: verificación de arranque durante una ventana estable, reintentos, rollback por digest exacto, verificación post-rollback, configuración global y por política, persistencia y notificación del resultado, y reconciliación de guards al arrancar.

## Requirements

### Requirement: Arranque sano verificado durante una ventana estable

Tras recrear un contenedor con la imagen nueva, Alloy MUST considerar que el contenedor
"arrancó bien" **solo** si su estado es `running` de forma **estable durante una ventana
temporal configurable** y sin que `RestartCount` aumente dentro de esa ventana. Si la imagen
define `HEALTHCHECK`, además el estado de salud MUST ser `healthy`. Los 3 sondeos puntuales
actuales (~21 s) NO son suficientes: un `crash-loop` posterior a la ventana MUST detectarse.

#### Scenario: Contenedor arranca sano sin HEALTHCHECK

- **Given** un contenedor recreado con la imagen nueva, sin `HEALTHCHECK` definido en la imagen
- **When** su estado permanece `running` durante toda la ventana y `RestartCount` no aumenta
- **Then** el arranque se considera sano
- **And** se aplica la política de limpieza de imagen antigua (si procede)

#### Scenario: Contenedor entra en crash-loop tras un arranque aparente

- **Given** un contenedor que arranca `running` pero reinicia repetidamente dentro de la ventana
- **When** `RestartCount` aumenta durante la ventana
- **Then** el arranque NO se considera sano
- **And** se activa el flujo de reintentos

#### Scenario: Imagen con HEALTHCHECK no sana

- **Given** un contenedor cuya imagen define `HEALTHCHECK`
- **When** durante la ventana el estado de salud es `unhealthy` (o nunca llega a `healthy`)
- **Then** el arranque NO se considera sano
- **And** se activa el flujo de reintentos

#### Scenario: Imagen con HEALTHCHECK sana

- **Given** un contenedor cuya imagen define `HEALTHCHECK`
- **When** durante la ventana el estado es `running` estable y la salud es `healthy`
- **Then** el arranque se considera sano

### Requirement: Reintentos de arranque hasta N configurables

Si el contenedor no arranca sano, Alloy MUST reintentar el arranque de la imagen nueva hasta
**N** veces (default **3**, configurable de forma global y con override por política),
esperando un intervalo configurable entre intentos. Al agotar los N intentos sin arranque
sano, MUST iniciar el rollback.

#### Scenario: Arranque sano en un intento posterior

- **Given** un contenedor que no arranca sano al primer intento
- **When** el arranque sano se consigue en el intento 2 (dentro de N)
- **Then** se detiene el flujo de reintentos
- **And** el resultado final es actualización correcta (sin rollback)

#### Scenario: Se agotan los N intentos

- **Given** un contenedor configurado con `max_start_attempts = 3`
- **When** los 3 intentos fallan la verificación de arranque sano
- **Then** se inicia el rollback a la imagen anterior
- **And** se registra en el historial el número de intentos realizados

#### Scenario: Override por política

- **Given** un contenedor con `max_start_attempts = 1` en su `UpdatePolicy` y un default global de 3
- **When** el primer intento falla la verificación de arranque sano
- **Then** NO se realizan más intentos (el override tiene prioridad)

### Requirement: Rollback a la imagen anterior por digest exacto

Antes de recrear un contenedor con la imagen nueva, Alloy MUST persistir el **digest/config
id exacto** de la imagen en ejecución. En caso de rollback MUST recrear el contenedor usando
ese digest exacto y MUST NOT emplear un tag frágil `rollback-<ts>`. La imagen anterior MUST
NOT eliminarse hasta confirmar que la nueva está sana.

#### Scenario: Rollback usa el digest exacto persistido

- **Given** una actualización fallida cuyo rollback se activa
- **When** Alloy recrea el contenedor con la imagen anterior
- **Then** la referencia usada es el digest/config id exacto persistido antes de actualizar
- **And** NO se usa un tag `rollback-<timestamp>`

#### Scenario: Imagen antigua retenida hasta confirmar la nueva sana

- **Given** un contenedor recién recreado con la imagen nueva
- **When** la verificación de arranque sano aún no ha concluido con éxito
- **Then** la imagen anterior NO se ha eliminado
- **And** solo tras confirmarse la nueva sana se aplica la limpieza de la imagen anterior

### Requirement: Verificación post-rollback y estado terminal

Tras el rollback, Alloy MUST verificar que la imagen anterior arranca sana (misma ventana
estable + HEALTHCHECK). Si la imagen anterior arranca sana → estado `rolled-back` y
notificación de nivel **warn**. Si la imagen anterior **tampoco** arranca → Alloy MUST dejar
el contenedor **parado**, con estado terminal `failed-stopped`, y emitir notificación de
nivel **error**.

#### Scenario: Rollback sano

- **Given** un rollback que recrea el contenedor con la imagen anterior
- **When** la imagen anterior arranca sana
- **Then** el estado final registrado es `rolled-back`
- **And** se emite una notificación de nivel `warn`
- **And** el contenedor queda en ejecución con la imagen anterior

#### Scenario: Rollback que tampoco arranca

- **Given** un rollback que recrea el contenedor con la imagen anterior
- **When** la imagen anterior tampoco arranca sano
- **Then** el contenedor se deja parado
- **And** el estado final registrado es `failed-stopped`
- **And** se emite una notificación de nivel `error`

#### Scenario: No hay reintentos infinitos tras el fallo terminal

- **Given** un contenedor en estado terminal `failed-stopped`
- **When** transcurren ciclos posteriores del worker
- **Then** Alloy NO vuelve a reintentar el arranque automáticamente ese contenedor
- **And** el contenedor permanece parado hasta intervención explícita del operador

### Requirement: Configuración de la guarda (global y por política)

Alloy MUST exponer en `GET/PUT /api/update-check/config` los campos de configuración
`update_max_start_attempts` (default 3), `update_start_attempt_interval_secs` y
`update_health_window_secs`, persistidos en settings. `UpdatePolicy` MUST aceptar los
overrides opcionales `max_start_attempts` y `health_window_secs`. Los overrides por política
MUST tener prioridad sobre los valores globales; si son `None`, MUST usarse el valor global.

#### Scenario: Defaults globales aplicados

- **Given** un settings sin los campos nuevos
- **When** se aplica una política sin overrides
- **Then** se usan los defaults (`update_max_start_attempts = 3`) y la ventana/intervalo por defecto

#### Scenario: Override por política con prioridad

- **Given** un settings con `update_max_start_attempts = 3` y una política con `max_start_attempts = 5`
- **When** el contenedor no arranca sano
- **Then** se realizan hasta 5 intentos (gana el override)

#### Scenario: Roundtrip de configuración vía API

- **Given** un `PUT /api/update-check/config` con `update_max_start_attempts = 4`
- **When** se consulta `GET /api/update-check/config`
- **Then** el valor devuelto es 4 y ha quedado persistido en settings

### Requirement: Persistencia y notificación del resultado del update

El historial de updates MUST registrar el resultado de cada fase, incluyendo `old_digest`,
`new_digest`, el número de intentos de arranque y el estado final (`updated`, `rolled-back`,
`failed-stopped`). El fin del proceso MUST emitir notificación: **warn** en rollback y
**error** en fallo terminal.

#### Scenario: Entrada de historial con intentos y digests

- **Given** una actualización que agotó los intentos y acabó en rollback sano
- **When** finaliza el proceso
- **Then** el historial registra `old_digest` y `new_digest`
- **And** registra el número de intentos realizados
- **And** registra el estado final `rolled-back`

#### Scenario: Notificación warn en rollback y error en fallo terminal

- **Given** un rollback sano
- **When** se notifica
- **Then** la notificación se emite con nivel `warn`
- **Given** un fallo terminal `failed-stopped`
- **When** se notifica
- **Then** la notificación se emite con nivel `error`

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

### Requirement: Overrides de la guarda en la política por defecto

Además de los overrides **globales** (`update_max_start_attempts`, `update_health_window_secs`)
y de los overrides **por contenedor** (`UpdatePolicy.max_start_attempts`,
`UpdatePolicy.health_window_secs`), el sistema MUST aceptar overrides de la **política por
defecto**:

- `Settings.default_max_start_attempts: Option<u32>`
- `Settings.default_health_window_secs: Option<u64>`

`GET /api/update-policies/default` MUST devolver ambos campos (resueltos desde settings) y
`PUT /api/update-policies/default` MUST persistirlos. Un valor `null` o ausente en el `PUT`
MUST interpretarse como «sin override». La ausencia de estos campos en settings almacenados
previos MUST ser tolerada (deserializan a `None`).

**Precedencia**: `override de política de contenedor` > `override de política por defecto` >
`global`. Los overrides de la política por defecto MUST aplicarse **solo** al *fallback* de los
contenedores **sin** política específica. Un contenedor **con** política que no fije su propio
override MUST seguir cayendo al **global**.

#### Scenario: Round-trip de los overrides de la política por defecto

- **Given** settings sin overrides de política por defecto
- **When** se hace `PUT /api/update-policies/default` con `max_start_attempts = 5` y `health_window_secs = 60`
- **Then** la respuesta y un `GET` posterior devuelven `max_start_attempts = 5` y `health_window_secs = 60`
- **And** los valores persisten tras recargar settings desde la base de datos

#### Scenario: Contenedor sin política usa el override por defecto

- **Given** un `Settings` con `update_max_start_attempts = 3` (global) y `default_max_start_attempts = 5`
- **And** un contenedor **sin** política específica
- **When** se resuelve la guarda de ese contenedor
- **Then** el máximo de intentos efectivo es **5** (el override por defecto, no el global)

#### Scenario: Contenedor con política conserva su override

- **Given** `default_max_start_attempts = 5` y una política de contenedor con `max_start_attempts = 7`
- **When** se resuelve la guarda de ese contenedor
- **Then** el máximo de intentos efectivo es **7** (gana el override de la política del contenedor)

#### Scenario: Contenedor con política sin override cae al global, no al default

- **Given** un `Settings` con `update_max_start_attempts = 3` (global) y `default_max_start_attempts = 5`
- **And** una política de contenedor **sin** `max_start_attempts`
- **When** se resuelve la guarda de ese contenedor
- **Then** el máximo de intentos efectivo es **3** (el global; el override por defecto no aplica a contenedores con política)

#### Scenario: Limpiar un override por defecto

- **Given** un `Settings` con `default_max_start_attempts = 5`
- **When** se hace `PUT /api/update-policies/default` con `max_start_attempts = null`
- **Then** el override queda en `None`
- **And** la guarda de un contenedor sin política vuelve a usar el global
