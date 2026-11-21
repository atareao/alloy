# Proposal

## Why

Las dependencias del frontend están desactualizadas. El issue #163 propone separar la actualización en dos fases: primero parches/menores de bajo riesgo, después los majors (que requieren verificación completa y se tratan en su propio change). Actualizar ahora los parches/menores reduce la deriva y el ruido de `pnpm outdated` antes de abordar los majors.

## What Changes

Actualizar dentro de los rangos ya declarados en `frontend/package.json` (sin subir de major):

- `antd` 6.6.4 → 6.7.1
- `react` / `react-dom` 19.2.7 → 19.3.1
- `vite` 8.3.6 → 8.3.8
- `oxlint` 1.74.0 → 1.94.0
- `@vitejs/plugin-react` 6.0.3 → 6.3.0
- `@testing-library/react` 16.3.2 → 16.3.3
- `@types/react` 19.2.17 → 19.3.0
- `@types/react-dom` 19.2.3 → 19.3.0
- Parches dentro de rango: `typescript` 6.0.2 → 6.0.3, `@types/node` 24.13.2 → 24.13.3, `vitest` 4.1.11 → 4.1.12, `@testing-library/jest-dom` 6.9.1 → 6.9.x.

Los majors (`vitest` 5, `jsdom` 31, `typescript` 7, `@testing-library/jest-dom` 8, `@types/node` 26) NO se tocan en este change; van en `deps-major`.

## Capabilities

### New Capabilities

- `frontend/deps`: política y verificación de actualización de dependencias del frontend.

### Modified Capabilities

Ninguna.

## Impact

- `frontend/package.json` (solo si algún rango necesita ajuste) y `frontend/pnpm-lock.yaml`.
- Sin cambios de código, API ni comportamiento esperado.
- Issue: #163 (fase 1 de 2).
