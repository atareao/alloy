# PLAN-005 — Exponer los defaults de updates por API

- **Issue**: #139 (https://github.com/atareao/alloy/issues/139)
- **Change OpenSpec**: `expose-update-defaults-api`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/expose-update-defaults-api` desde `develop`; PR a `develop`.

## Objetivo

Eliminar el hardcodeo de los defaults de updates en el frontend exponiéndolos por API desde el punto único (las consts del backend).

## Decisiones (operador)

- Endpoint **nuevo** `GET /api/update-check/defaults` (struct propio); sin tocar el GET/PUT existente.
- Solo los **3 canónicos**: `cron`, `pull_timeout_secs`, `check_interval_ms`.
- El formulario **depende de la API** (placeholder/etiqueta/fallback); se ajusta el escenario del spec.

## Problema

- Frontend hardcodea: cron init `ConfigPage.tsx:72` + placeholder `:954`; timeout init `:82` + fallback `:985` + etiqueta `:978-981`.
- Backend fuente de verdad: `models.rs:279-283`.
- El spec actual exige mostrar los canónicos ANTES de cargar de la API → hay que modificar ese requisito.

## Alcance

1. `backend/src/models.rs`: struct `UpdateDefaults { cron, pull_timeout_secs, check_interval_ms }` (serde) desde las consts.
2. `backend/src/admin.rs`: `get_update_defaults_h` + ruta `GET /api/update-check/defaults`.
3. `frontend/src/types.ts`: tipo `UpdateDefaults`.
4. `frontend/src/components/ConfigPage.tsx`: cargar defaults; usarlos en placeholder/etiqueta/fallback; eliminar literales.
5. Spec: `MODIFIED` del requisito "Consistencia de los defaults entre backend y UI".

## Fuera de alcance (posible follow-up)

- Defaults de la **guarda** (3/10/30) en el frontend.
- `check_interval_ms`: se expone, no se edita en UI.

## Plan TDD

### RED
- Backend: test del endpoint nuevo ⇒ 200 con los 3 canónicos.
- Frontend: test de placeholder/etiqueta/fallback usando los defaults de la API (mock con valores distintos para ser probatorio).

### GREEN
- Struct + handler + ruta; tipo TS + consumo en `ConfigPage.tsx`.

### REFACTOR / verificación
- `cd backend && just check` + `cargo test`.
- `cd frontend && pnpm exec tsc -b` + `pnpm run build` + tests.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Backend y frontend en verde.
- [ ] `docs/expose-update-defaults-api.md`.
- [ ] `openspec archive expose-update-defaults-api` + limpieza.
- [ ] PR a `develop` mergeado e issue #139 cerrado.

## Referencias

- #125 / PR #129; #135 / PR #136.
- Spec delta: `openspec/changes/expose-update-defaults-api/specs/updates/update-check/spec.md`.
- `docs/reconcile-update-defaults.md:65-66`; `models.rs:279-283`.
