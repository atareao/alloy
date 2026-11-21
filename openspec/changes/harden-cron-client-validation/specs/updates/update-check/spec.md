# updates/update-check Delta

## MODIFIED Requirements

### Requirement: Validación del cron en el formulario antes de enviar

El formulario de configuración MUST validar la expresión cron **antes** de enviarla. La validación MUST
comprobar, en este orden:
- que haya exactamente **5 campos** (`minuto hora día-del-mes mes día-de-semana`);
- que cada campo sea `*`, un valor, un rango `a-b`, o cualquiera de ellos con un paso `/n`, admitiendo
  **listas** separadas por `,` (p. ej. `a,b`, `a-b,c`);
- que los valores estén **en rango** por campo: minuto `0–59`, hora `0–23`, día-del-mes `1–31`, mes `1–12`
  (o nombre `JAN..DEC`, insensible a mayúsculas), día-de-semana `0–7` (o nombre `SUN..SAT`);
- que los **pasos** sean `> 0` y los **rangos** no estén invertidos (`a <= b`).

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

#### Scenario: Un rango invertido o un paso cero se detectan en el cliente

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce `30-10 * * * *` (rango invertido) o `*/0 * * * *` (paso cero)
- **Then** se muestra un error junto al campo
- **And** **no** se realiza la petición `PUT`

#### Scenario: Expresiones con rangos, pasos y nombres son válidas

- **Given** el formulario con el auto-update habilitado
- **When** el usuario introduce un cron válido como `*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *` o
  `0 6 * * 1-5`
- **Then** no se muestra error de validación y se envía el `PUT`

#### Scenario: La UI muestra el error del backend si este rechaza un cron aceptado por el cliente

- **Given** un cron que pasa la validación del cliente
- **When** el backend responde `400` con `{"error": ...}` al `PUT`
- **Then** la UI muestra ese mensaje de error del backend
