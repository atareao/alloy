# Proposal

## Why

El stage `frontend-builder` del `Dockerfile` **hardcodea** la versión de pnpm
(`RUN npm install -g pnpm@11.21.0`), duplicando el campo `packageManager` de
`frontend/package.json` (`"pnpm@11.21.0"`). Hoy coinciden, pero nada garantiza que sigan
coincidiendo: si se actualiza `packageManager` (o el pin del Dockerfile) sin tocar el otro, el build
de producción y el entorno de CI/desarrollo usan versiones de pnpm distintas sin que nada lo
detecte. El workflow de CI ya deriva la versión de `packageManager` (`pnpm/action-setup` sin
`version`), así que el Dockerfile es el último punto que duplica esa versión.

## What Changes

- `Dockerfile`: el stage `frontend-builder` deriva la versión de pnpm de
  `frontend/package.json` (`packageManager`) en lugar de fijarla. Sin pin literal `pnpm@<x.y.z>`.
- `ci.yml`: nuevo paso de consistencia de pnpm (análogo al de Node) que (a) exige que el
  `Dockerfile` derive la versión de `packageManager` y (b) falla si vuelve a hardcodear un literal
  `pnpm@<x.y.z>`.
- Documentación: `docs/pnpm-single-package-manager.md` y `docs/frontend-ci-verification.md`
  actualizados; se retira de `docs/ci-node-24.md` el follow-up que este cambio resuelve.

No hay cambios de versión efectiva (sigue siendo `pnpm@11.21.0`) ni de código de aplicación.

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `ci/frontend-verification`: se añade un requisito (ADDED) que exige que el `Dockerfile` derive la
  versión de pnpm de `packageManager` —igual que ya exige para Node y para el propio workflow— y que
  CI lo verifique. Encaja en el propósito de la capability: coherencia de toolchain entre CI y el
  build de producción.

## Impact

- `Dockerfile` (stage `frontend-builder`: una línea de instalación de pnpm; se adelanta el `COPY` de
  `package.json`).
- `.github/workflows/ci.yml` (nuevo paso en el job `frontend`).
- `docs/pnpm-single-package-manager.md`, `docs/frontend-ci-verification.md`, `docs/ci-node-24.md`.
