# PLAN-020 — Actualizar dependencias frontend: parches/menores (#163, fase 1)

**Issue:** #163
**Change OpenSpec:** `openspec/changes/deps-minor/`
**Rama:** `feature/deps-minor` (desde `develop`)

## Objetivo

Subir parches y menores de las dependencias del frontend sin cruzar ningún major, dejando la suite verde y el lockfile sincronizado. Los majors se abordan en `deps-major` (fase 2).

## Mapa de cambios

- `frontend/pnpm-lock.yaml` (y `frontend/package.json` solo si hace falta ajustar rangos).

## Tareas

1. Ejecutar `pnpm update` en `frontend/`.
2. Revisar `pnpm outdated` para confirmar que solo quedan majors.
3. Ejecutar `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint`, `pnpm run build`.
4. Verificar `pnpm install --frozen-lockfile`.
5. Revisar diff, commit y PR.

## Criterios de aceptación

- [x] `antd` 6.7.1, `react`/`react-dom` 19.3.1, `vite` 8.3.8, `oxlint` 1.94.0.
- [x] Ningún major actualizado en este change.
- [x] `tsc -b`, test, lint y build en 0; ≥174 tests.
- [x] `pnpm install --frozen-lockfile` sin cambios.
- [x] Diff limitado a `frontend/package.json` + `pnpm-lock.yaml` + artefactos OpenSpec/plan.
