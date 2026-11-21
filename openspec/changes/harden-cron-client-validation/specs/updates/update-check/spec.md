# updates/update-check Delta

## ADDED Requirements

### Requirement: Edición del preset «Cada domingo» con día-de-semana válido

El preset «Cada domingo» del formulario MUST generar una expresión cron cuyo día-de-semana sea válido
para el backend (crate `cron`, que usa día-de-semana `1–7` con `1` = domingo y `0` inválido). El preset
MUST producir `0 0 * * SUN` (o un equivalente como `0 0 * * 1`) y MUST pasar la validación de cliente.

#### Scenario: El preset «Cada domingo» produce un cron válido

- **Given** el formulario con el auto-update habilitado
- **When** el usuario elige el preset «Cada domingo»
- **Then** la expresión resultante (p. ej. `0 0 * * SUN`) no muestra error de validación
- **And** el backend la acepta (día-de-semana `1–7`, no `0`)

## MODIFIED Requirements

### Requirement: Validación del cron en el formulario antes de enviar

El formulario de configuración MUST validar la expresión cron **antes** de enviarla, de forma coherente
con la gramática del backend (crate `cron`), que sigue siendo la **autoridad final**. La validación MUST
comprobar, en este orden:

- que haya exactamente **5 campos** (`minuto hora día-del-mes mes día-de-semana`) tras normalizar;
- que cada campo sea una **lista** separada por `,` cuyos elementos sean `*`, `?` (solo en día-del-mes y
  día-de-semana), un **valor**, o un **rango** `a-b`, admitiendo un **paso** `/n` tras `*`, un valor o un
  rango (p. ej. `*/n`, `a/n`, `a-b/n`);
- que los **valores y rangos** estén en rango por campo: minuto `0–59`, hora `0–23`, día-del-mes `1–31`,
  mes `1–12`, día-de-semana `1–7` (nótese `1–7`, **no** `0–7`); y que los **rangos** no estén invertidos
  (`a <= b`);
- que los **pasos** sean enteros con `1 <= n <= máximo` del campo (minuto 59, hora 23, día-del-mes 31,
  mes 12, día-semana 7);
- que los **nombres** (solo en mes y día-de-semana) sean alfabéticos de longitud ≥ 3; la validación de
  nombres es deliberadamente **permisiva** (superset de los alias de la crate) para no bloquear nombres
  válidos: un nombre que el backend no reconozca lo rechazará el backend.

Si es inválida MUST mostrar un **error inline** junto al campo y MUST **NO** enviar el `PUT`
`/api/update-check/config`. La validación del cliente es un pre-chequeo; el backend sigue siendo la
**autoridad final** (si el backend rechazara una expresión que el cliente aceptó, la UI MUST mostrar el
mensaje de error del backend).

#### Scenario: Cron malformado muestra error inline y no envía el PUT

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce un cron malformado (p. ej. `not a cron`) y pulsa guardar
- **Then** se muestra un error junto al campo
- **And** **no** se realiza la petición `PUT /api/update-check/config`

#### Scenario: Cron válido de 5 campos no bloquea el guardado

- **Given** el formulario con un cron válido de 5 campos (p. ej. `0 6 * * *`)
- **When** el usuario pulsa guardar
- **Then** no se muestra error de validación y se envía el `PUT`

#### Scenario: Un valor fuera de rango se detecta en el cliente

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce `99 0 * * *` y pulsa guardar
- **Then** se muestra un error junto al campo
- **And** **no** se realiza la petición `PUT`

#### Scenario: Un día-de-semana `0` se detecta en el cliente

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce `0 0 * * 0` y pulsa guardar
- **Then** se muestra un error junto al campo
- **And** **no** se realiza la petición `PUT`

#### Scenario: Un rango invertido o un paso fuera de rango se detectan en el cliente

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce `30-10 * * * *` (rango invertido) o `*/0 * * * *` o `*/60 * * * *`
  (paso cero o por encima del máximo del campo)
- **Then** se muestra un error junto al campo
- **And** **no** se realiza la petición `PUT`

#### Scenario: Expresiones con rangos, pasos, listas, nombres y `?` son válidas

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce un cron válido como `*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *`,
  `0 6 * * 1-5`, `0 6 ? * *`, `0 6 * * SUN`, `0 6 * * MON-FRI` o `0 6 1,15 * *`
- **Then** no se muestra error de validación y se envía el `PUT`

#### Scenario: Nombres largos de mes/día-de-semana no se bloquean

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce `0 6 * * MONDAY` o `0 6 1 JANUARY *`
- **Then** no se muestra error de validación y se envía el `PUT`

#### Scenario: La UI muestra el error del backend si este rechaza un cron aceptado por el cliente

- **Given** un cron que pasa la validación del cliente
- **When** el backend responde `400` con `{"error": ...}` al `PUT`
- **Then** la UI muestra ese mensaje de error del backend
