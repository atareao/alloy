# Tasks

## TDD Checklist

### Actualización

- [x] Actualizar `vitest` a `^5.1.1`, `jsdom` a `^31.0.1`, `typescript` a `~7.0.2`, `@testing-library/jest-dom` a `^8.0.0`.
- [x] Añadir `@testing-library/dom` `^10.4.2` como devDependency.
- [x] Subir `engines.node` a `^24.15`.
- [x] Ejecutar `pnpm install` y resolver cualquier incompatibilidad de peers.

### Ajustes derivados

- [x] Ajustar `tsconfig*.json` si TypeScript 7 lo requiere (documentar).
- [x] Ajustar `test-setup.ts` o tests si vitest 5 / jsdom 31 cambian comportamiento (documentar).

### Verificación

- [x] `pnpm exec tsc -b` → 0.
- [x] `pnpm run test` → 0 (≥174 tests).
- [x] `pnpm run lint` → 0.
- [x] `pnpm run build` → 0.
- [x] `pnpm install --frozen-lockfile` → 0 sin modificar el lockfile.
- [x] Commit con gitmoji (`deps`) y PR a `develop`.
