# Proposal

## Why

Follow-up de #135/#139/#148. `check_interval_ms` (intervalo en ms entre verificaciones de digest de cada
contenedor) es un default canónico que el backend **ya acepta y devuelve** (`GET`/`PUT`
`/api/update-check/config`, expuesto además en `GET /api/update-check/defaults`), pero la **UI no lo
muestra ni lo edita**: el usuario no puede ajustarlo.

## What Changes

- **Frontend** (`types.ts` + `ConfigPage.tsx`):
  - Añadir `check_interval_ms` a `interface UpdateCheckConfig`.
  - Nuevo estado `checkIntervalSecs` (en **segundos**), cargado desde la config (`check_interval_ms` /
    1000) y usado en un `InputNumber` (min 1 s) en el bloque de revisión.
  - Al guardar, convertir a **ms** (`secs * 1000`) en el `PUT`.
  - Placeholder/etiqueta desde `GET /api/update-check/defaults` (`check_interval_ms` / 1000), sin literales.
- **Tests**: `ConfigPage.test.tsx` — cargar (2 s desde 2000 ms), guardar (5 s ⇒ `check_interval_ms: 5000`)
  y etiqueta/placeholder desde la API.

## Capabilities

### Modified Capabilities

- `updates/update-check`: nuevo comportamiento de UI (edición del intervalo de comprobación). El contrato
  del backend **no cambia** (ya soporta `check_interval_ms`).

### New Capabilities

- Ninguna.

## Impact

- **Frontend**: `types.ts`, `ConfigPage.tsx` (+ tests). **Solo frontend**.
- **Backend**: sin cambios.
- **API**: sin cambios (el campo ya existe en el GET/PUT y en `/defaults`).
- **Fuera de alcance**: validación cliente del cron por rangos (#150).
