# Proposal

## Why

`ci.yml` valida **solo el backend**. No instala Node, no ejecuta `tsc`, no corre los 72 tests de
Vitest, no construye con Vite y no aplica oxlint. Cualquier regresión del frontend —un error de
tipos, un test roto, un import que rompe el build— llega a `develop` sin que nada la detecte.

El change `pnpm-single-package-manager` (reciente) dejó escritos los contratos del frontend con
pnpm, pero **sin ninguna vigilancia automática**: la verificación de aquel PR fue manual, y solo
porque se ejecutó a mano. Un contrato que nadie comprueba se degrada.

## What Changes

- Nuevo job `frontend` en `.github/workflows/ci.yml`, **en paralelo** al job `test` existente.
- El job instala con `pnpm install --frozen-lockfile` y ejecuta, en pasos separados: type-check
  (`tsc -b`), tests (`vitest run`), build (`vite build`) y lint (oxlint).
- La versión de pnpm se toma del campo `packageManager` de `frontend/package.json`; el workflow
  **no** duplica el pin.
- Node se fija a la misma versión mayor que el stage `frontend-builder` del `Dockerfile`.
- El job de backend **no se toca**.

## Capabilities

### New Capabilities

- `ci/frontend-verification`: verificación automática del frontend (tipos, tests, build, lint) en
  cada PR, con la misma toolchain que el build de producción.

### Modified Capabilities

Ninguna. No cambia el comportamiento del producto, y ningún spec existente describe la
automatización del frontend (`frontend/*` y `security/dependency-hygiene` describen el producto,
no qué comprueba la CI).

## Impact

**Ficheros afectados**

- `.github/workflows/ci.yml` — job `frontend` añadido; el job `test` queda intacto.

**Sin cambios en**: `backend/`, `frontend/src/`, `Dockerfile`, `release*.yml`.

**Coste**: ~1 minuto de CI extra por PR, ejecutado en paralelo, sin alargar la ruta crítica.
