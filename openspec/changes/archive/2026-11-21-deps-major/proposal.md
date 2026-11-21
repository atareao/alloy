# Proposal

## Why

Fase 2 de #163. Tras actualizar parches/menores (`deps-minor`, PR #177), quedan los majors del frontend: `vitest` 5, `jsdom` 31, `typescript` 7 y `@testing-library/jest-dom` 8. Son cambios con riesgo de compatibilidad y requieren verificación completa.

`@testing-library/jest-dom` 8 declara peer obligatorio `@testing-library/dom >=10 <11`, hoy presente solo de forma transitiva; conviene declararlo explícitamente.

## What Changes

- `vitest` `^4.1.12` → `^5.1.1`.
- `jsdom` `^29.1.1` → `^31.0.1`.
- `typescript` `~6.0.3` → `~7.0.2`.
- `@testing-library/jest-dom` `~6.9.1` → `^8.0.0`.
- Añadir `@testing-library/dom` `^10.4.2` como devDependency (peer de jest-dom 8).
- `engines.node` `^24` → `^24.15` (jsdom 31 exige `>=24.15.0`; el chequeo de CI compara solo el major, así que sigue alineado con el Dockerfile `node:24`).
- `@types/node` se mantiene en `^24.13.2` (coherente con el runtime Node 24; no se sube a 26).

Si algún major exige ajustes de configuración (p. ej. `tsconfig` con TypeScript 7), se aplican en este change y se documentan.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `frontend/deps`: la política de actualización de dependencias se extiende a los majors con verificación completa.

## Impact

- `frontend/package.json`, `frontend/pnpm-lock.yaml` y, si TypeScript 7 lo exige, `frontend/tsconfig*.json`.
- Posible ajuste en tests si cambia el comportamiento de vitest 5 o jsdom 31.
- Issue: #163 (fase 2 de 2).
