# PLAN-021 — Actualizar dependencias frontend: majors (#163, fase 2)

**Issue:** #163
**Change OpenSpec:** `openspec/changes/deps-major/`
**Rama:** `feature/deps-major` (desde `develop`)

## Objetivo

Subir los majors pendientes del frontend (vitest 5, jsdom 31, TypeScript 7, jest-dom 8) con verificación completa, dejando la suite verde y el lockfile consistente.

## Mapa de cambios

- `frontend/package.json` (versiones, devDependency nueva, `engines.node`).
- `frontend/pnpm-lock.yaml`.
- `frontend/tsconfig*.json` y/o `frontend/src/test-setup.ts` solo si los majors lo exigen.

## Tareas

1. Aplicar los bumps de major y añadir `@testing-library/dom`.
2. Subir `engines.node` a `^24.15`.
3. `pnpm install` y resolver peers.
4. Corregir lo que TypeScript 7 o vitest 5 rompan (documentando cada ajuste).
5. Ejecutar `tsc -b`, test, lint, build y `--frozen-lockfile`.
6. Revisar diff, commit y PR.

## Criterios de aceptación

- [x] `vitest` 5.x, `jsdom` 31.x, `typescript` 7.0.x, `@testing-library/jest-dom` 8.x.
- [x] `@testing-library/dom` declarado como devDependency.
- [x] `@types/node` sigue en 24.x; `engines.node` `^24.15`.
- [x] `tsc -b`, test (≥174), lint y build en 0; `--frozen-lockfile` sin cambios.
- [x] Cualquier ajuste de código/config justificado y documentado.
- [x] Diff limitado a `frontend/` + artefactos OpenSpec/plan.
