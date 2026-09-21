# Proposal

## Why

La verificación de tipos en frontend usaba `npx tsc --noEmit`, que es menos estricto que `tsc -b` (project build mode). Esto permitió que un error de compilación (propiedad `_batchCurrentItem` no existente en interfaz) pasara desapercibido hasta el build real.

## What Changes

Cambiar en AGENTS.md la línea de type checking de frontend:
- Antes: `npx tsc --noEmit` (mandatory during GREEN/REFACTOR steps)
- Después: `npx tsc -b` (mandatory during GREEN/REFACTOR steps)

## Capabilities

Sin cambios de comportamiento. `skip_specs: true`.

## Impact

Solo `AGENTS.md` — una línea modificada.