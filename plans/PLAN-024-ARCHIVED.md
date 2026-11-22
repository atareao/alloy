# PLAN-024 — Resolver warnings react(set-state-in-effect) (#179)

**Issue:** #179
**Change OpenSpec:** `openspec/changes/lint-set-state-in-effect/`
**Rama:** `feature/lint-set-state-in-effect` (desde `develop`)

## Objetivo

Eliminar los 5 warnings de `react(set-state-in-effect)` de oxlint 1.94 sin cambiar el comportamiento observable, y activar `--deny-warnings` en el script `lint`.

## Mapa de cambios

- `frontend/src/useConfigSync.ts` (nuevo hook de sincronización desde props).
- `frontend/src/components/TelegramCard.tsx`, `ValetCard.tsx`, `MatrixCard.tsx` (quitar `useEffect`, usar el hook).
- `frontend/src/components/DashboardPage.tsx` (reseteos de logs a `handleLogs`).
- `frontend/src/App.tsx` (disable justificado del falso positivo asíncrono).
- `frontend/package.json` (script `lint` con `--deny-warnings`).
- Tests: `ConfigPage.test.tsx`, `DashboardPage.test.tsx`.

## Tareas

1. Cubrir con tests el comportamiento de sincronización desde props y de logs.
2. Crear el hook `useConfigSync` y refactorizar los tres `*Card`.
3. Mover los reseteos de logs al event handler.
4. Documentar el falso positivo de `App.tsx`.
5. Activar `--deny-warnings` en el script `lint`.
6. Verificar `test`, `tsc -b`, `lint` (0 warnings) y `build`.

## Criterios de aceptación

- [x] `pnpm run lint` termina con 0 warnings.
- [x] El script `lint` usa `--deny-warnings`.
- [x] Comportamiento preservado: precarga, token vacío, switch activado y logs.
- [x] `test`, `tsc -b`, `lint` y `build` en 0; ≥190 tests.
- [x] Diff limitado a `frontend/` + artefactos OpenSpec/plan.
