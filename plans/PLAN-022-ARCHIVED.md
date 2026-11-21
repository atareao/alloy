# PLAN-022 — Elipsis en nombre/estado del contenedor (#166, parte 1)

**Issue:** #166
**Change OpenSpec:** `openspec/changes/container-row-ellipsis/`
**Rama:** `feature/container-row-ellipsis` (desde `develop`)

## Objetivo

Que el nombre y el estado del contenedor se trunquen con elipsis real (con tooltip) en móvil y escritorio, eliminando el truncado manual con `slice`. Sin tocar la imagen (no se muestra en la fila).

## Mapa de cambios

- `frontend/src/components/ContainerRow.tsx` (líneas ~135-160 y ~266-290: nombre, estado y progreso).
- `frontend/src/components/ContainerRow.test.tsx` (test de truncado móvil).
- `frontend/src/components/DashboardPage.tsx` (solo si el nombre del stack necesita ajuste de ancho).

## Tareas

1. Reescribir el test de truncado para exigir elipsis real y tooltip.
2. Sustituir los `slice` manuales por elipsis de Ant Design con tooltip.
3. Ajustar `minWidth`/`flex` para que el truncado sea efectivo sin comprimir los elementos no truncables.
4. Verificar la suite, `tsc`, lint y build.

## Criterios de aceptación

- [x] No queda ningún `slice(` para truncar nombre, estado ni progreso en `ContainerRow.tsx`.
- [x] Nombre y estado usan elipsis real con tooltip.
- [x] Badge de actualización, spinner y botón de expansión siguen visibles con nombres largos.
- [x] La imagen no se renderiza en la fila.
- [x] Suite ≥174 tests, `tsc -b`, lint y build en 0.
- [x] Diff limitado a `frontend/` + artefactos OpenSpec/plan.
