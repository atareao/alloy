# Tasks: ui-fixes-v4

## 1. Eliminar background override del inner div

- [x] 1.1 En `DashboardPage.tsx` — `renderGroup` mobile (línea 334): cambiar `background: "var(--ant-color-bg-layout)"` a `background: "transparent"`

## 2. Aumentar gap del grid

- [x] 2.1 En `ContainerTable.tsx` — grid mobile (línea 140): cambiar `gap: 1` a `gap: 8`

## 3. Verificación

- [x] 3.1 Ejecutar `npx vitest run` — 18 passed
- [x] 3.2 Ejecutar `npx tsc --noEmit` — sin errores