# Proposal

## Why

Follow-up de #135/#139. En #139 se expusieron por API los **3 canónicos** (`cron`, `pull_timeout_secs`,
`check_interval_ms`) y la UI dejó de hardcodearlos. Pero los defaults de la **guarda de arranque** siguen
hardcodeados en el frontend (`ConfigPage.tsx:131-133` init `3/10/30`, y fallbacks `v == null ? 3|10|30`
en los `onChange`), duplicando las consts del backend
(`DEFAULT_UPDATE_MAX_START_ATTEMPTS=3`, `DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS=10`,
`DEFAULT_UPDATE_HEALTH_WINDOW_SECS=30`).

## What Changes

- **Backend** (`models.rs`): ampliar el struct `UpdateDefaults` (endpoint `GET /api/update-check/defaults`)
  con 3 campos nuevos — `update_max_start_attempts`, `update_start_attempt_interval_secs`,
  `update_health_window_secs` — poblados **desde las consts**. Es **aditivo**: no toca el GET/PUT de config.
- **Frontend** (`types.ts` + `ConfigPage.tsx`): ampliar la `interface UpdateDefaults` y usar los valores
  de la API para el **fallback** de los 3 campos de la guarda (al vaciarlos), eliminando los literales.
- **Tests**: backend (endpoint devuelve también los 3 de la guarda); frontend (fallback desde la API con
  valores distintos de los canónicos ⇒ probatorio).

## Capabilities

### Modified Capabilities

- `updates/update-check`:
  - `Endpoint de defaults de configuración de updates`: el objeto devuelto incluye también los defaults de
    la guarda.
  - `Consistencia de los defaults entre backend y UI`: el frontend toma de la API también los defaults de
    la guarda (fallback).

### New Capabilities

- Ninguna.

## Impact

- **Backend**: `models.rs` (`UpdateDefaults` + `canonical()`), test del endpoint en `admin.rs`.
- **Frontend**: `types.ts`, `ConfigPage.tsx` (+ tests).
- **API**: aditivo (el endpoint `/defaults` gana 3 campos); el GET/PUT de config no cambia.
