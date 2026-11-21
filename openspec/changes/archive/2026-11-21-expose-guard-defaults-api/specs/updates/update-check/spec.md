# updates/update-check Delta

## MODIFIED Requirements

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
