# Proposal

## Why

Varios componentes con lógica no trivial no tienen test colocado: `DashboardPage.tsx`, `ContainerRow.tsx`, `ContainerTable.tsx`, `ContainerActions.tsx` e `InspectModal.tsx`. Las acciones de contenedor (start/stop/restart/remove/pull) son las de mayor riesgo operativo. Además, `frontend/src/ErrorBoundary.test.tsx` está descolocado: el componente vive en `frontend/src/components/ErrorBoundary.tsx`, incumpliendo la convención de tests colocados.

## What Changes

- Mover `ErrorBoundary.test.tsx` a `frontend/src/components/ErrorBoundary.test.tsx` y ajustar el import a `./ErrorBoundary`.
- Añadir tests colocados para `DashboardPage`, `ContainerRow`, `ContainerTable`, `ContainerActions` e `InspectModal`.
- Cubrir renderizado, callbacks de acciones, estados de carga/error, filtros y modales.
- No se cambia código de producción salvo que un test revele un bug; en ese caso se documenta y se corrige en el mismo change con justificación.

## Capabilities

### New Capabilities

- `frontend/test-coverage`: tests colocados para los componentes del dashboard y sus acciones.

### Modified Capabilities

Ninguna.

## Impact

- Solo tests en `frontend/src/components/` y el movimiento del test de `ErrorBoundary`.
- Sin cambios de API, dependencias ni comportamiento.
- `ContainerRow` no renderiza la imagen; se documenta en el spec y se cubre en `InspectModal`. Mostrarla en la fila requeriría un change incremental.
- Issue: #162.
