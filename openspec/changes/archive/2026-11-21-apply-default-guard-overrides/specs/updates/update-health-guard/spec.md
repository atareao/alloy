# updates/update-health-guard Delta

## ADDED Requirements

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
