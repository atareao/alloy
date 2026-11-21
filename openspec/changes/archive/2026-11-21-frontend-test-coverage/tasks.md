# Tasks

## TDD Checklist

### Tests colocados

- [x] Mover `ErrorBoundary.test.tsx` a `frontend/src/components/` y ajustar el import
- [x] Crear `ContainerActions.test.tsx` (callbacks, running/exited, batch, stack)
- [x] Crear `ContainerRow.test.tsx` (renderizado, expansión, acciones, badge)
- [x] Documentar que ContainerRow no renderiza imagen; la imagen se cubre en InspectModal.
- [x] Crear `ContainerTable.test.tsx` (búsqueda, filtros, check all, grupos, batch)
- [x] Crear `InspectModal.test.tsx` (loading, error, vacío, datos, cierre)
- [x] Crear `DashboardPage.test.tsx` (filtros, acciones API, inspección, borrado)

### Verificación

- [x] `npx vitest run` verde
- [x] `pnpm exec tsc -b` limpio
- [x] `pnpm run lint` limpio
- [x] `pnpm run build` limpio
- [x] `rg` confirma que no queda `frontend/src/ErrorBoundary.test.tsx`
