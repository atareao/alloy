# PLAN-003 — Reconciliar defaults de updates (cron, pull_timeout_secs, check_interval_ms)

- **Issue**: #135 (https://github.com/atareao/alloy/issues/135)
- **Change OpenSpec**: `reconcile-update-defaults`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/reconcile-update-defaults` desde `develop`; PR a `develop`.

## Objetivo

Dejar un **único origen** para los defaults de configuración de updates y alinear backend (API + ejecución), docs y frontend.

## Valores canónicos (decididos por el operador)

- **cron**: `0 6 * * *` (todos los días a las 6:00 am).
- **pull_timeout_secs**: `600`.
- **check_interval_ms**: `2000`.

## Problema

- cron: API/UI `0 */6 * * *` (admin.rs:25, ConfigPage.tsx:72) vs. worker real `0 0 * * *` (scheduler.rs:51, handlers.rs:1532).
- pull_timeout_secs: docs/etiqueta `1800` (models.rs:247,393, ConfigPage.tsx:978) vs. ejecución `600` (7 sitios: stacks.rs:193, scheduler.rs:292,332,480, handlers.rs:701,949,1709) vs. form inicial 600 (ConfigPage.tsx:82).
- check_interval_ms: `unwrap_or(2000)` en scheduler.rs:139 y handlers.rs:1229.
- Sin const compartida (la guarda sí la tiene: models.rs:272-288).

## Alcance

1. Consts en `backend/src/models.rs`: `DEFAULT_UPDATE_CHECK_CRON="0 6 * * *"`, `DEFAULT_PULL_TIMEOUT_SECS=600`, `DEFAULT_CHECK_INTERVAL_MS=2000`.
2. Sustituir literales/fallbacks en `models.rs`, `db.rs`, `admin.rs`, `scheduler.rs`, `handlers.rs`, `stacks.rs`.
3. Alinear `frontend/src/components/ConfigPage.tsx` (cron inicial/placeholder, pull_timeout inicial/fallback, etiqueta UI).
4. Actualizar tests backend y `ConfigPage.test.tsx`.

## Fuera de alcance (posible follow-up)

- Validación de cron en el PUT (`admin.rs:77-79`).
- Exponer defaults por API para que el frontend no los hardcodee.

## Ficheros implicados

- `backend/src/{models.rs,db.rs,admin.rs,stacks.rs,workers/scheduler.rs,updates/handlers.rs}`.
- `frontend/src/components/ConfigPage.tsx` (+ tests).

## Plan TDD

### RED
- Backend: tests de fallback canónico para cron/pull_timeout/check_interval.
- Frontend: `ConfigPage.test.tsx` ajustado a `0 6 * * *` y `600`.

### GREEN
- Consts + sustituciones; alineación del frontend.

### REFACTOR / verificación
- `cd backend && just check` + `cargo test`.
- `cd frontend && pnpm run build` + `pnpm exec tsc -b` + lint/tests.
- `rg` para confirmar que no quedan literales duplicados en producción.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Backend y frontend en verde.
- [ ] `docs/reconcile-update-defaults.md`.
- [ ] `openspec archive reconcile-update-defaults` + limpieza.
- [ ] PR a `develop` mergeado e issue #135 cerrado.

## Referencias

- #125 / PR #129; follow-ups #130/#131, #132/#133.
- Spec delta: `openspec/changes/reconcile-update-defaults/specs/updates/update-check/spec.md`.
- `docs/update-rollback-guard.md:81`.
