# updates/update-check Delta

## MODIFIED Requirements

### Requirement: PullRestartStack aplica la actualización o falla de forma visible

Al ejecutar la política `PullRestartStack`, Alloy MUST aplicar el `pull` de la imagen y la
recreación del servicio de compose **vía la API de Docker (Bollard)**, sin depender del
binario `docker` ni de un fichero de compose visible en el contenedor. El éxito MUST
considerarse tal **solo si el contenedor pasa la verificación de arranque sano**
(update-health-guard). Cualquier fallo MUST quedar registrado con nivel `ERROR` (nunca en
silencio). El éxito MUST emitir notificación, persistir el digest de la imagen en ejecución,
marcar `has_update=false` e incrementar el contador de actualizaciones.

#### Scenario: Servicio de stack actualizado con éxito

- **Given** un contenedor de compose con update disponible y política `PullRestartStack`
- **When** el pull por digest, la recreación vía API y la verificación de arranque sano
  terminan con éxito
- **Then** se persiste como `last_remote_digest` el config digest resuelto de forma fiable
  (o, si la resolución fue degradada, el image id del contenedor recreado)
- **And** se marca `has_update=false`
- **And** se emite una notificación equivalente a la de `PullRestart`
- **And** el contador de contenedores actualizados aumenta en 1
- **And** se registra un `INFO` de éxito

#### Scenario: Fallo del pull de la imagen

- **Given** un `PullRestartStack` cuyo `pull_image` por digest falla
- **When** se ejecuta la política
- **Then** se registra un `ERROR` indicando el contenedor y la imagen
- **And** NO se cambia `has_update` ni `last_remote_digest`
- **And** `update_in_progress` se limpia de forma simétrica

#### Scenario: Fallo de la recreación vía API

- **Given** un `PullRestartStack` cuyo `recreate_container` devuelve `Err`
- **When** se ejecuta la política
- **Then** se registra un `ERROR` con la causa
- **And** `has_update` permanece sin cambios

#### Scenario: Recreación sin arranque sano dispara la guarda

- **Given** un `PullRestartStack` cuya recreación devuelve `Ok` pero el contenedor no arranca sano
- **When** la verificación de arranque sano falla
- **Then** se aplica la guarda de update (reintentos → rollback por digest → estado final)
- **And** el resultado se refleja en el historial y en las notificaciones correspondientes

#### Scenario: Resolución degradada durante un stack update

- **Given** un registry que no expone el config digest real (resultado degradado)
- **When** el servicio se recrea con éxito y arranca sano
- **Then** NO se persiste el manifest digest como `last_remote_digest`
- **And** se registra un `WARN` indicando modo degradado
- **And** no se vuelve a recrear el mismo servicio por el mismo manifest en ciclos
  sucesivos (anti-bucle por manifest ya aplicado)

## ADDED Requirements

### Requirement: La aplicación de políticas integra la guarda de arranque

Al aplicar una política que recrea el contenedor (`PullRestart`/`PullRestartStack`) con
`rollback_on_failure` activo, Alloy MUST delegar la detección de fallo y el rollback en la
guarda `update-health-guard` en lugar del sondeo puntual y el tag `rollback-<ts>` actuales.
Los resultados (`rolled-back`, `failed-stopped`) MUST exponerse en el historial y en las
notificaciones, y MUST NOT quedar solo en el log como `warn`.

#### Scenario: Update que falla y acaba en rollback sano

- **Given** un contenedor con `rollback_on_failure = true` cuya imagen nueva no arranca sano
- **When** se agotan los intentos y el rollback por digest arranca sano
- **Then** el estado final es `rolled-back`
- **And** se emite notificación `warn`
- **And** el historial refleja el resultado (no solo un log)

#### Scenario: Update y rollback que fallan

- **Given** un contenedor cuya imagen nueva no arranca y cuya imagen anterior tampoco arranca
- **When** el rollback falla la verificación de arranque sano
- **Then** el contenedor queda parado con estado `failed-stopped`
- **And** se emite notificación `error`
- **And** el historial refleja el estado terminal
