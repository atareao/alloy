# HANDOFF

## Estado

Cola **148 → 149 → 150** completada, mergeada y **liberada**. `develop` == `main` ==
`0d63204 chore: release v0.26.0` (tag **v0.26.0**, imagen publicada).

### Features de la cola
- **#148** `expose-guard-defaults-api` — PR #151; `UpdateDefaults` ampliado (aditivo) con los 3 defaults de
  la guarda de arranque; UI sin literales. Docs: `docs/expose-guard-defaults-api.md`.
- **#149** `expose-check-interval-ui` — PR #152; `check_interval_ms` editable en segundos en la UI
  (+ estabilización del test flaky del cron de #141). Docs: `docs/expose-check-interval-ui.md`.
- **#150** `harden-cron-client-validation` — PR #153; validación cliente del cron por rangos/sintaxis
  alineada con la crate `cron 0.15` (día-semana `1–7`, `?`, nombres permisivos, tope de paso) y **preset
  «Cada domingo» arreglado** (`0 0 * * 0` → `0 0 * * SUN`). Docs: `docs/harden-cron-client-validation.md`.

### Release
- PR #154 (`develop` → `main`) mergeado; `Prepare Release` → bump minor **0.25.0 → 0.26.0**, tag
  **v0.26.0**, back-merge a `develop`. `Release` → imagen publicada.

## Baselines de verificación

- Backend: `cargo test` → **432 OK**; `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `tsc -b`, `lint`, `build` limpios.

## Tarea pendiente

- **Ninguna abierta.** Planes `PLAN-007/008/009` archivados.
- **co1 redeployado a v0.26.0** (Dockge): imagen `ghcr.io/atareao/alloy:latest` (digest manifest
  `0258b306…`), contenedor recreado y sano (OIDC/Docker OK, HTTP 200). Procedimiento: `docs/deploy-co1.md`.
