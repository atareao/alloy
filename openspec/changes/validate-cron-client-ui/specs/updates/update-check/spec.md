# updates/update-check Delta

## ADDED Requirements

### Requirement: Validación del cron en el formulario antes de enviar

El formulario de configuración MUST validar la expresión cron **antes** de enviarla. Si es inválida
(no tiene **5 campos** o algún campo no respeta la estructura permitida —dígitos y los operadores
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
