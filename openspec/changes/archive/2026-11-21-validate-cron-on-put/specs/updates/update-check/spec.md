# updates/update-check Delta

## ADDED Requirements

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
