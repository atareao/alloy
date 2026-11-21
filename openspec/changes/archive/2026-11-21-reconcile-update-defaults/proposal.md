# Proposal

## Why

Los defaults de configuración de updates están **duplicados y en conflicto**:

- **cron**: la API y la UI anuncian `0 */6 * * *` (`backend/src/admin.rs:25`,
  `frontend/src/components/ConfigPage.tsx:72`), pero el worker real cae a `0 0 * * *`
  (`backend/src/workers/scheduler.rs:51`, `updates/handlers.rs:1532`). Con settings sin cron, la UI
  muestra cada 6 h pero el scheduler corre a medianoche.
- **pull_timeout_secs**: los doc-comments y la etiqueta de UI dicen `1800` (`models.rs:247,393`,
  `ConfigPage.tsx:978`), pero la ejecución real usa `600` en 7 sitios y el form inicial `600`.
- **check_interval_ms**: `unwrap_or(2000)` duplicado (`scheduler.rs:139`, `handlers.rs:1229`).

La guarda de arranque ya tiene el patrón correcto (consts fuente única, `models.rs:272-288`); este
change lo replica para el resto de defaults de updates.

## What Changes

- **Único origen** en `backend/src/models.rs`:
  - `DEFAULT_UPDATE_CHECK_CRON: &str = "0 6 * * *"` (diario a las 6:00 am),
  - `DEFAULT_PULL_TIMEOUT_SECS: u64 = 600`,
  - `DEFAULT_CHECK_INTERVAL_MS: u64 = 2000`.
- Sustituir **todos** los literales/fallbacks por esas consts en `models.rs`, `db.rs`, `admin.rs`,
  `scheduler.rs`, `updates/handlers.rs` y `stacks.rs`.
- **Alinear el frontend** (`ConfigPage.tsx`) a los canónicos: estado inicial del cron, placeholder,
  `pull_timeout_secs` inicial y fallback al vaciar, y la etiqueta de UI ("Default: 600 (10 min)").
- Actualizar los tests que fijan estos defaults (backend y `ConfigPage.test.tsx`).

## Capabilities

### Modified Capabilities

- `updates/update-check`: los defaults de `cron`, `pull_timeout_secs` y `check_interval_ms` pasan a
  declararse en un **único origen** y se aplican de forma consistente en API, scheduler, workers y UI.

### New Capabilities

- Ninguna.

## Impact

- **Backend**: `models.rs` (consts + `Default`/serde), `db.rs`, `admin.rs` (GET fallback),
  `workers/scheduler.rs`, `updates/handlers.rs`, `stacks.rs`.
- **Frontend**: `ConfigPage.tsx` (+ tests).
- **API**: sin cambios de contrato (mismos endpoints/campos); cambia el **valor** del fallback de `cron`
  a `0 6 * * *`.
- **Datos**: sin migración (la tabla `settings` es clave/valor genérica, sin DEFAULT por columna).
- **Dependencias**: ninguna.
