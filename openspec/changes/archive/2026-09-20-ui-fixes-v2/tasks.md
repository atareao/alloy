# Tasks: ui-fixes-v2

## 1. Card llena toda la celda (height: 100%)

- [x] 1.1 En `DashboardPage.tsx` — `renderGroup` mobile: añadir `style={{ height: '100%' }}` a la Card (línea 318)

## 2. Rejilla visible entre celdas

- [x] 2.1 En `ContainerTable.tsx` línea 141: cambiar `background: 'var(--ant-color-bg-container)'` a `background: 'var(--ant-color-border-secondary)'`

## 3. Verificación

- [x] 3.1 Ejecutar `npx vitest run` para tests frontend — ✅ 18 passed
- [x] 3.2 Ejecutar `npx tsc --noEmit` para type checking — ✅ sin errores
- [x] 3.3 Ejecutar `cargo test` para tests backend — ✅ 181 passed (4 integración requieren Docker)