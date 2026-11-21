# Proposal

## Why

El backend ya declara los defaults de updates como **consts de punto único** (#135), pero el frontend
sigue **hardcodeándolos** (`ConfigPage.tsx:72` cron init, `:954` placeholder, `:82`/`:985` timeout,
`:978-981` etiqueta). Cualquier cambio futuro del valor debe replicarse a mano → deriva.

El requisito `Consistencia de los defaults entre backend y UI` (`openspec/specs/updates/update-check/`)
hoy exige mostrar los canónicos **antes** de cargar de la API; consumirlos de la API **cambia** ese
comportamiento, por lo que el requisito se modifica.

## What Changes

- **Backend**: nuevo struct `UpdateDefaults { cron, pull_timeout_secs, check_interval_ms }`
  (`models.rs`, poblado desde las consts) + handler `get_update_defaults_h` + ruta
  `GET /api/update-check/defaults` (`admin.rs::routes`). Sin tocar el GET/PUT existente.
- **Frontend**: cargar los defaults de ese endpoint y usarlos para el **placeholder** del cron, la
  **etiqueta** del timeout y el **fallback** al vaciar; **eliminar** los literales duplicados. Añadir el
  tipo TS `UpdateDefaults`.
- **Spec**: `MODIFIED` del requisito de consistencia — el formulario toma los defaults de la API
  (placeholder/etiqueta/fallback) y no los hardcodea.

## Capabilities

### Modified Capabilities

- `updates/update-check`: nuevo endpoint de defaults; el formulario consume los defaults de la API en
  lugar de literales incrustados.

### New Capabilities

- Ninguna.

## Impact

- **Backend**: `models.rs` (struct `UpdateDefaults`, reutiliza consts), `admin.rs` (handler + ruta).
- **Frontend**: `ConfigPage.tsx` (placeholder/etiqueta/fallback desde la API), `types.ts`.
- **API**: nuevo `GET /api/update-check/defaults` (aditivo; no altera el contrato del GET/PUT).
- **Datos**: sin migración.
- **Fuera de alcance**: los defaults de la **guarda** (3/10/30), que siguen hardcodeados (posible follow-up).
