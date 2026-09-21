# Tasks: ui-fixes-v3

## 1. Card con borde visible

- [x] 1.1 En `DashboardPage.tsx` — `renderGroup` mobile: eliminar `bordered={false}` del Card (usar valor por defecto = bordered)

## 2. Grid sin background propio

- [x] 2.1 En `ContainerTable.tsx` línea 141: cambiar `background: 'var(--ant-color-border-secondary)'` a `background: 'transparent'`

## 3. Verificación

- [x] 3.1 Ejecutar `npx vitest run` — ✅ 18 passed
- [x] 3.2 Ejecutar `npx tsc --noEmit` — ✅ sin errores