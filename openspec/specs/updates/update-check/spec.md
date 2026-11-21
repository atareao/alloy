# updates/update-check Specification

## Purpose
Definir el comportamiento observable y fiable del worker `update-check`: que las
políticas de actualización se apliquen de verdad (incluida `PullRestartStack`), que los
fallos de resolución remota se registren sin pisar el estado, que todo check de digest
tenga un desenlace logueado y que las consultas a registry se dedupliquen por imagen.

## Requirements

### Requirement: PullRestartStack aplica la actualización o falla de forma visible

Al ejecutar la política `PullRestartStack`, Alloy MUST aplicar el `pull` de la imagen y la
recreación del servicio de compose **vía la API de Docker (Bollard)**, sin depender del
binario `docker` ni de un fichero de compose visible en el contenedor. El éxito MUST
considerarse tal **solo si el contenedor pasa la verificación de arranque sano**
(update-health-guard) **cuando la política tiene `rollback_on_failure` activo**; con `rollback_on_failure=false` se conserva el comportamiento previo (sin verificación ampliada ni rollback), por decisión explícita de diseño. Cualquier fallo MUST quedar registrado con nivel `ERROR` (nunca en
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

### Requirement: El scheduler no silencia errores de resolución remota

Cuando `check_remote_digest_with_docker` devuelve `Err`, el worker `update-check` MUST
registrar el error con el nombre del contenedor y la imagen, y MUST NOT sobrescribir el
estado de actualización a `false` por un fallo de resolución.

#### Scenario: Registry inalcanzable

- **Given** un contenedor cuya imagen no puede resolverse remotamente (red/registry)
- **When** el worker comprueba si hay update
- **Then** se registra un `WARN`/`ERROR` con el nombre e imagen
- **And** `has_update` conserva su valor previo (no se fuerza a `false`)
- **And** se continúa con el siguiente contenedor

#### Scenario: Primera comprobación sin estado previo

- **Given** un contenedor sin `has_update` previo y un error de resolución
- **When** el worker comprueba si hay update
- **Then** se registra el error
- **And** el contenedor no se marca como actualizado ni se aplica política alguna

### Requirement: Todo check de digest registra un desenlace

`check_remote_digest_*` MUST registrar exactamente un desenlace por invocación, con un
nivel acorde al resultado: `INFO` para éxito, `WARN` para resultado degradado y `ERROR`
para fallo.

#### Scenario: Éxito por HTTP directo (docker.io, ghcr.io)

- **Given** una imagen resoluble por HTTP con token
- **When** finaliza el check
- **Then** se registra `INFO ... OK manifest_digest=... config_digest=...`

#### Scenario: Éxito vía Docker daemon (registry desconocido)

- **Given** un registry desconocido resuelto por el daemon
- **When** finaliza el check con config digest obtenido
- **Then** se registra `INFO ... OK` con ambos digests
- **And** no se omite el log de desenlace por un `return` temprano

#### Scenario: Resultado degradado (config digest = manifest digest)

- **Given** un registry que falla el flujo HTTP de auth tras el daemon
- **When** se usa el fallback que iguala config y manifest digest
- **Then** se registra un `WARN` indicando resultado DEGRADED y que la comparación puede
  dar un falso positivo
- **And** el desenlace queda registrado (no silencioso)

#### Scenario: Fallo total del check

- **Given** un registry que no responde o rechaza toda autenticación
- **When** el check no puede obtener el config digest
- **Then** se registra un `ERROR` con la causa (status HTTP, realm, etc.)

### Requirement: Deduplicación de comprobaciones por imagen

Dentro de un ciclo de `update-check`, Alloy MUST consultar el registry como máximo una vez
por referencia de imagen única, reutilizando el resultado entre todos los contenedores que
comparten esa imagen.

#### Scenario: Varios contenedores comparten imagen

- **Given** tres contenedores con la misma imagen `wordpress:fpm-alpine`
- **When** se ejecuta el ciclo de check
- **Then** el registry se consulta una sola vez para esa imagen
- **And** los tres contenedores aplican la misma decisión de update

#### Scenario: Distintos tags de la misma base

- **Given** contenedores con `postgres:17` y `postgres:18-alpine`
- **When** se ejecuta el ciclo
- **Then** se realizan dos consultas independientes (referencias distintas)

### Requirement: Los helpers de persistencia del scheduler no paniquean

Los helpers de acceso a la base de datos y a locks usados por el worker `update-check` MUST
NOT usar `unwrap()` sobre `db_pool.get()` ni sobre `mutex.lock()`. Ante pool cerrado o
mutex envenenado MUST registrar `ERROR` (o `WARN` cuando proceda) y continuar, sin panic.

#### Scenario: Pool de base de datos cerrado

- **Given** un `db_pool.get()` que devuelve `Err` (pool agotado o cerrado)
- **When** un helper de persistencia (`update_has_update`, `append_update_history`,
  `check_times`, `save_settings`) intenta obtener conexión
- **Then** se registra un `ERROR` con el contexto
- **And** no se produce panic
- **And** el ciclo del worker continúa con el siguiente contenedor

#### Scenario: Mutex envenenado

- **Given** un mutex de conexión envenenado (poisoned)
- **When** un helper intenta `lock()`
- **Then** se registra un `ERROR` y no se produce panic

#### Scenario: Fallo al cargar el mapa de digests

- **Given** que `load_last_remote_digest_map` no puede preparar la consulta o el lock
- **When** se carga el mapa al inicio del ciclo
- **Then** se devuelve un mapa vacío (comportamiento actual para prepare/query)
- **And** el fallo del lock también se registra sin panic

### Requirement: La política Pull no repite la descarga degradada

Cuando la resolución remota es degradada, la política `Pull` MUST NOT volver a descargar la
misma imagen en ciclos sucesivos mientras el manifest no cambie. Esta supresión MUST NOT
persistir `last_remote_digest` (la semántica de `Pull` de no avanzar el digest se mantiene).

#### Scenario: Pull degradado aplicado una vez

- **Given** un contenedor con política `Pull` y resolución degradada
- **When** se aplica el pull en un ciclo
- **Then** en el siguiente ciclo, con el mismo manifest degradado, no se vuelve a descargar
- **And** `last_remote_digest` NO se persiste

#### Scenario: Manifest degradado nuevo

- **Given** un contenedor con un manifest degradado ya aplicado
- **When** el manifest remoto degradado cambia
- **Then** se vuelve a descargar (no se suprime)

#### Scenario: Recuperación del registry

- **Given** un contenedor cuya resolución pasa de degradada a fiable
- **When** se ejecuta el ciclo
- **Then** la supresión no aplica y se recalcula `needs_update` con el config digest fiable

### Requirement: El fin de batch expone contadores reales

Cuando termina un batch de check (`check_and_apply_all`), el evento SSE de fin
(`update-progress` con `done = true`) MUST transportar los contadores reales acumulados:
`checked` (comprobados), `updatable` (con actualización disponible), `uptodate` (al día),
`updated` (aplicados con éxito), `failed` (fallos al aplicar) y `errors` (errores al comprobar).
MUST NOT emitir el evento de fin con todos los contadores a cero cuando se procesaron
contenedores. La limpieza posterior de la caché de progreso MUST NOT sobrescribir el evento de
fin con ceros.

#### Scenario: Evento de fin con contadores reales

- **Given** un batch que procesó 10 contenedores, 3 con actualización disponible, 0 errores y 0 fallos
- **When** finaliza el batch
- **Then** el evento `update-progress` con `done = true` lleva `checked = 10`, `updatable = 3`, `uptodate = 7`, `updated`/`failed`/`errors` coherentes

#### Scenario: Batch sin contenedores

- **Given** un batch sin contenedores que procesar
- **When** finaliza el batch
- **Then** el evento de fin lleva todos los contadores a 0 (es el único caso legítimo de ceros)

#### Scenario: Limpieza de caché sin pisar el resumen

- **Given** un batch que acaba con contadores reales
- **When** se limpia la caché de progreso (`reset_progress_cache`)
- **Then** el evento de fin con contadores reales no se reemite con ceros

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

### Requirement: Único origen de los defaults de configuración de updates

Los defaults de `cron` (auto-update), `pull_timeout_secs` y `check_interval_ms` MUST declararse **una
sola vez** como constantes en `backend/src/models.rs`, y todos los puntos de uso (impl `Default`, serde
defaults, lectura SQLite, GET API, scheduler, workers y flujos manuales) MUST referenciarlas. No MUST
quedar literales duplicados de estos defaults en código de producción.

Valores canónicos:

- `DEFAULT_UPDATE_CHECK_CRON = "0 6 * * *"` (todos los días a las 6:00).
- `DEFAULT_PULL_TIMEOUT_SECS = 600`.
- `DEFAULT_CHECK_INTERVAL_MS = 2000`.

#### Scenario: Config sin cron usa el default canónico en API y scheduler

- **Given** unos settings sin `update_check_cron` definido
- **When** se consulta `GET /api/update-check/config`
- **Then** el `cron` devuelto es `0 6 * * *`
- **And** el scheduler usa el mismo default `0 6 * * *` (no `0 0 * * *`)

#### Scenario: pull_timeout_secs sin definir usa 600

- **Given** unos settings sin `pull_timeout_secs`
- **When** cualquier flujo de pull (scheduler o update manual) necesita el timeout
- **Then** usa `600` (constante), no un literal disperso

#### Scenario: check_interval_ms sin definir usa 2000

- **Given** unos settings sin `check_interval_ms`
- **When** el worker/loop lo consulta
- **Then** usa `2000` (constante)

### Requirement: Consistencia de los defaults entre backend y UI

El frontend MUST obtener los defaults de updates desde la API (`GET /api/update-check/defaults`) y MUST
**NO** hardcodear esos valores. El **placeholder** del cron, la **etiqueta** del timeout y el
**fallback** al vaciar el campo `pull_timeout_secs` MUST reflejar los valores devueltos por la API. El
formulario no MUST depender de literales incrustados para estos defaults. El **fallback** al vaciar los
campos de la **guarda** (`update_max_start_attempts`, `update_start_attempt_interval_secs`,
`update_health_window_secs`) MUST reflejar asimismo los valores de la API.

#### Scenario: El formulario de settings arranca con los canónicos

- **Given** la página de configuración con los defaults cargados de `GET /api/update-check/defaults`
- **When** el usuario observa el campo de cron y el de `pull_timeout_secs`
- **Then** el placeholder del cron y la etiqueta del timeout muestran los valores devueltos por la API
  (los canónicos por defecto: `0 6 * * *` y `600 (10 min)`)

#### Scenario: Vaciar el timeout no cambia el default mostrado

- **Given** el campo `pull_timeout_secs` con valor y los defaults cargados de la API
- **When** el usuario lo vacía
- **Then** el fallback aplicado es el `pull_timeout_secs` devuelto por la API

#### Scenario: Vaciar un campo de la guarda usa el default de la API

- **Given** los campos de la guarda con valor y los defaults cargados de la API
- **When** el usuario vacía el campo `update_max_start_attempts`
- **Then** el fallback aplicado es el `update_max_start_attempts` devuelto por la API

### Requirement: Validación de la expresión cron al guardar la configuración

`PUT /api/update-check/config` MUST validar el campo `cron` antes de persistir. Una expresión
inválida, **vacía**, o que no tenga **exactamente 5 campos** MUST rechazarse con **400** y un cuerpo
`{"error": <mensaje>}`, sin modificar el estado persistido ni los settings en memoria.

La validación MUST realizarse mediante un helper de **punto único** `pub fn is_valid_cron(expr: &str)
-> bool` (en `backend/src/timezone.rs`), que exige 5 campos y comprueba el parseo con la crate `cron`
(prefijando `"0 "` internamente). El formato canónico es de **5 campos** (p. ej. `0 6 * * *`).

#### Scenario: Cron inválido devuelve 400 y no persiste

- **Given** unos settings con un cron válido guardado
- **When** se envía `PUT /api/update-check/config` con `cron = "not a cron"`
- **Then** la respuesta es **400** con `{"error": ...}`
- **And** los settings persistidos y en memoria **no cambian**

#### Scenario: Cron vacío devuelve 400

- **Given** el handler PUT de configuración
- **When** se envía `cron = ""`
- **Then** la respuesta es **400** con `{"error": ...}`

#### Scenario: Cron con 6 campos devuelve 400

- **Given** el handler PUT de configuración
- **When** se envía `cron = "0 0 6 * * *"` (6 campos)
- **Then** la respuesta es **400** con `{"error": ...}` (solo se acepta el formato canónico de 5 campos)

#### Scenario: Cron válido de 5 campos se guarda

- **Given** el handler PUT de configuración
- **When** se envía `cron = "0 6 * * *"`
- **Then** la respuesta es **200** con el `UpdateCheckConfig` guardado
- **And** `next_run_at` se calcula con el cron indicado

#### Scenario: El formulario muestra el error del backend

- **Given** la página de configuración con el auto-update habilitado
- **When** el usuario guarda con un cron inválido y el backend responde 400
- **Then** la UI muestra el mensaje de error devuelto por el backend (no un mensaje genérico)

### Requirement: Validación del cron al importar la configuración

`POST /api/admin/import` MUST validar el campo `update_check_cron` del `Settings` importado **antes**
de mutar el estado o persistir. Si el cron viene **presente y no vacío** y no es una expresión válida
de 5 campos, MUST rechazarse con **400** `{"error": msg}` sin modificar el estado en memoria ni el
persistido. Un cron ausente (`None`) o **vacío** representa "sin override" y MUST aceptarse. La
validación MUST delegar en el mismo helper `is_valid_cron`.

#### Scenario: Importar un settings con cron inválido devuelve 400 y no persiste

- **Given** unos settings persistidos con un cron válido
- **When** se envía `POST /api/admin/import` con un `settings` cuyo `update_check_cron` es `"not a cron"`
- **Then** la respuesta es **400** con `{"error": ...}`
- **And** el estado en memoria y el persistido **no cambian**

#### Scenario: Importar un settings sin cron (o vacío) se acepta

- **Given** el handler de importación
- **When** se importa un `settings` con `update_check_cron = None` (o `""`)
- **Then** la respuesta es **200** y el settings se aplica (sin override de cron)

### Requirement: Helper de validación de cron de punto único

La aplicación MUST exponer un único helper (`is_valid_cron`) para decidir si una expresión cron es
válida en el formato canónico de 5 campos. Todos los puntos que **validan** cron (el PUT de la
configuración y la importación) MUST delegar en él en lugar de duplicar el prefijado `"0 "` y el
parseo.

#### Scenario: El helper acepta 5 campos válidos y rechaza el resto

- **Given** el helper `is_valid_cron`
- **When** se le pasa `"0 6 * * *"` → **true**
- **And** se le pasa `"*/15 * * * *"` → **true**
- **And** se le pasa `""`, `"not a cron"` o `"0 0 6 * * *"` (6 campos) → **false**

### Requirement: Endpoint de defaults de configuración de updates

La API MUST exponer los defaults canónicos de la configuración de updates mediante
`GET /api/update-check/defaults`, que devuelve un objeto
`{ "cron": <string>, "pull_timeout_secs": <u64>, "check_interval_ms": <u64>,
"update_max_start_attempts": <u32>, "update_start_attempt_interval_secs": <u64>,
"update_health_window_secs": <u64> }` poblado **desde las consts** de `models.rs`
(`DEFAULT_UPDATE_CHECK_CRON`, `DEFAULT_PULL_TIMEOUT_SECS`, `DEFAULT_CHECK_INTERVAL_MS`,
`DEFAULT_UPDATE_MAX_START_ATTEMPTS`, `DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS`,
`DEFAULT_UPDATE_HEALTH_WINDOW_SECS`). No MUST haber literales de esos valores en el handler.

#### Scenario: El endpoint devuelve los canónicos

- **Given** el servidor en marcha
- **When** se consulta `GET /api/update-check/defaults`
- **Then** responde **200** con `cron = "0 6 * * *"`, `pull_timeout_secs = 600`,
  `check_interval_ms = 2000` (las consts)

#### Scenario: El endpoint devuelve también los defaults de la guarda

- **Given** el servidor en marcha
- **When** se consulta `GET /api/update-check/defaults`
- **Then** responde **200** con `update_max_start_attempts = 3`,
  `update_start_attempt_interval_secs = 10`, `update_health_window_secs = 30` (las consts)

### Requirement: Validación del cron en el formulario antes de enviar

El formulario de configuración MUST validar la expresión cron **antes** de enviarla. Si es inválida
(no tiene **5 campos** o algún campo no respeta la estructura permitida —dígitos, letras (nombres de mes/día) y los operadores
`* , - /`—), MUST mostrar un **error inline** junto al campo y MUST **NO** enviar el `PUT`
`/api/update-check/config`. La validación del cliente es un pre-chequeo ligero; el backend sigue siendo
la autoridad final.

#### Scenario: Cron malformado muestra error inline y no envía el PUT

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce un cron malformado (p. ej. `not a cron`) y pulsa guardar
- **Then** se muestra un error junto al campo
- **And** **no** se realiza la petición `PUT /api/update-check/config`

#### Scenario: Cron válido de 5 campos no bloquea el guardado

- **Given** el formulario con un cron válido de 5 campos (p. ej. `0 6 * * *`)
- **When** el usuario pulsa guardar
- **Then** no se muestra error de validación y se envía el `PUT`

### Requirement: Normalización de expresiones cron de 6 campos en el formulario

El formulario MUST normalizar una expresión cron de **6 campos** eliminando el campo de **segundos**
(primero) y reduciéndola al formato canónico de **5 campos**, antes de validarla y de enviarla. El
backend NO cambia: sigue aceptando solo 5 campos.

#### Scenario: Un cron de 6 campos se normaliza a 5 al salir del campo

- **Given** el formulario con el campo cron
- **When** el usuario introduce `0 0 6 * * *` (6 campos) y sale del campo
- **Then** el valor del campo pasa a `0 6 * * *` (5 campos)
- **And** al guardar, el `PUT` envía `cron = "0 6 * * *"`

#### Scenario: Normalizar y validar en el guardado sin volver a 400

- **Given** el formulario con un cron de 6 campos válido subyacentemente
- **When** el usuario pulsa guardar
- **Then** se normaliza a 5 campos antes del envío y el `PUT` no recibe un 400 por formato
