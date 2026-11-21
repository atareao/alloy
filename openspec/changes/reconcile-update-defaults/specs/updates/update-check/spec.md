# updates/update-check Delta

## ADDED Requirements

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

El frontend MUST mostrar/valores por defecto coherentes con los canónicos del backend: el estado inicial
del formulario, el placeholder del cron, el valor inicial de `pull_timeout_secs`, su fallback al vaciar
y la etiqueta de UI MUST coincidir con `0 6 * * *` y `600`.

#### Scenario: El formulario de settings arranca con los canónicos

- **Given** la página de configuración recién abierta (antes de cargar de la API)
- **When** el usuario observa los campos de cron y `pull_timeout_secs`
- **Then** el cron inicial/placeholder es `0 6 * * *`
- **And** `pull_timeout_secs` inicial es `600` y su etiqueta indica el default `600 (10 min)`

#### Scenario: Vaciar el timeout no cambia el default mostrado

- **Given** el campo `pull_timeout_secs` con valor
- **When** el usuario lo vacía
- **Then** el fallback aplicado es `600` (no `1800`)
