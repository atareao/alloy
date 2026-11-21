# PLAN-007 — Exponer los defaults de la guarda por API

- **Issue**: #148 (https://github.com/atareao/alloy/issues/148)
- **Change OpenSpec**: `expose-guard-defaults-api`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/expose-guard-defaults-api` desde `develop`; PR a `develop`.

## Objetivo

Eliminar el hardcodeo en el frontend de los defaults de la guarda de arranque, exponiéndolos por API desde el punto único (las consts del backend).

## Decisiones

- Ampliar (aditivo) el endpoint existente `GET /api/update-check/defaults` con 3 campos: `update_max_start_attempts`, `update_start_attempt_interval_secs`, `update_health_window_secs`.
- Poblarlos **desde las consts** (`DEFAULT_UPDATE_MAX_START_ATTEMPTS=3`, `DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS=10`, `DEFAULT_UPDATE_HEALTH_WINDOW_SECS=30`). Cero literales.
- El GET/PUT de config NO cambia.

## Problema

- Frontend hardcodea la guarda: init `ConfigPage.tsx:131-133` (`3/10/30`) y fallbacks en `onChange` (`v == null ? 3|10|30`).
- Backend fuente de verdad: `models.rs:272-276`.

## Alcance

1. `backend/src/models.rs`: ampliar `UpdateDefaults` + `canonical()` con los 3 campos.
2. `backend/src/admin.rs`: actualizar el test del endpoint.
3. `frontend/src/types.ts`: ampliar `interface UpdateDefaults`.
4. `frontend/src/components/ConfigPage.tsx`: fallback de los 3 campos de la guarda desde `defaults`.
5. Spec: `MODIFIED` de `Endpoint de defaults de configuración de updates` y `Consistencia de los defaults entre backend y UI`.

## Fuera de alcance

- Cambiar los límites (min/max) de los InputNumber de la guarda.
- `check_interval_ms` (issue #149) y validación por rangos del cron (issue #150).

## Plan TDD

### RED
- Backend: el test del endpoint exige también `3/10/30`.
- Frontend: mock de `/defaults` con valores distintos ⇒ vaciar un campo de la guarda aplica el valor de la API.

### GREEN
- Struct + canonical; tipo TS + consumo en `ConfigPage.tsx`.

### REFACTOR / verificación
- `cd backend && just check` + `cargo test`.
- `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Backend y frontend en verde.
- [ ] `docs/expose-guard-defaults-api.md`.
- [ ] `openspec archive expose-guard-defaults-api` + limpieza.
- [ ] PR a `develop` mergeado e issue #148 cerrado.

## Referencias

- #135 / PR #136; #139 / PR #140.
- Spec delta: `openspec/changes/expose-guard-defaults-api/specs/updates/update-check/spec.md`.
- `models.rs:272-276`; `ConfigPage.tsx:131-133,1115,1137,1154`.
