# updates/update-health-guard Delta

## ADDED Requirements

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
