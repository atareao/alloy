# HANDOFF

## Estado

Cola **148 → 149 → 150** completada y mergeada en `develop`. Cada ítem con su ciclo SDD → aprobación →
TDD → auditoría → PR a `develop` → archive de OpenSpec → cierre de issue.

- **#148** `expose-guard-defaults-api` — PR #151 mergeado; `UpdateDefaults` ampliado con los 3 defaults de
  la guarda; UI sin literales. Docs: `docs/expose-guard-defaults-api.md`.
- **#149** `expose-check-interval-ui` — PR #152 mergeado; `check_interval_ms` editable en segundos en la UI
  (+ estabilización del test flaky del cron de #141). Docs: `docs/expose-check-interval-ui.md`.
- **#150** `harden-cron-client-validation` — PR #153 mergeado; validación cliente del cron por rangos/
  sintaxis alineada con la crate `cron 0.15` (día-semana `1–7`, `?`, nombres permisivos, tope de paso) y
  **preset «Cada domingo» arreglado** (`0 0 * * 0` → `0 0 * * SUN`). Docs:
  `docs/harden-cron-client-validation.md`.

**HEAD:** `develop` = `dbe1cc4` (main sigue en v0.25.0). Planes `PLAN-007/008/009` archivados.

## Baselines de verificación

- Backend: `cargo test` → **432 OK**; `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `tsc -b`, `lint`, `build` limpios.

## Tarea pendiente

- **Release**: no se ha subido nada a `main`. `develop` acumula las 3 features de la cola sobre `main`
  (v0.25.0). El release (bump minor → tag → imagen) se dispara **mergeando `develop` → `main`** (PR con
  merge commit); el workflow hace el versionado/tag y el back-merge a `develop`. **Pendiente de decisión
  del operador.**
- No hay otras tareas abiertas en la cola.
