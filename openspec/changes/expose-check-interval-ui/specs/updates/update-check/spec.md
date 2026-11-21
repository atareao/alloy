# updates/update-check Delta

## ADDED Requirements

### Requirement: Edición del intervalo de comprobación en el formulario

El formulario de configuración de updates MUST permitir editar el intervalo de comprobación
(`check_interval_ms`) en **segundos**. Al cargar la configuración MUST mostrar el valor recibido
convertido a segundos (`check_interval_ms / 1000`) y al enviar MUST convertirlo de nuevo a **milisegundos**
(`segundos * 1000`) en el `PUT /api/update-check/config`. La **etiqueta** y el **placeholder** del control
MUST reflejar el default devuelto por `GET /api/update-check/defaults` (convertido a segundos), sin
literales incrustados. El valor MUST ser de al menos **1 segundo**.

#### Scenario: Cargar el intervalo desde la configuración

- **Given** la configuración con `check_interval_ms = 2000`
- **When** se abre el formulario de update
- **Then** el control del intervalo muestra `2` (segundos)

#### Scenario: Guardar el intervalo lo convierte a milisegundos

- **Given** el formulario con el intervalo puesto a `5` segundos
- **When** el usuario guarda
- **Then** el `PUT /api/update-check/config` envía `check_interval_ms = 5000`

#### Scenario: La etiqueta refleja el default de la API

- **Given** los defaults cargados de `GET /api/update-check/defaults` con `check_interval_ms = 2000`
- **When** el usuario observa el control del intervalo antes de modificarlo
- **Then** la etiqueta/placeholder muestran el default en segundos (`2 s`) tomado de la API
