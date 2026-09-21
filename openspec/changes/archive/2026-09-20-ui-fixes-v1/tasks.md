# Tasks — ui-fixes-v1

## Task Checklist

### 1. Layout: Eliminar marco/borde alrededor de la app
- [x] **GREEN**: Modificar `App.tsx` — cambiar `Layout` a `background: transparent`, eliminar `border: 'none'`

### 2. Layout: Reemplazar tabs por Sidebar
- [x] **GREEN**: Modificar `App.tsx`:
  - [x] Importar `Layout.Sider`, `Menu` de antd
  - [x] Importar iconos de navegación (Dashboard, History, Settings, Logout)
  - [x] Reestructurar layout: `Layout` → `Layout.Sider` + `Layout` (Header + Content)
  - [x] Configurar Sider con `collapsible`, `breakpoint="md"`
  - [x] Crear `Menu` con items: Dashboard, Historial, Config, Salir
  - [x] Mapear clics de Menu al estado `view`
  - [x] Limpiar Header: solo logo + nombre usuario, sin navegación

### 3. Dashboard Mobile: Compactar filas y diferenciar del fondo
- [x] **GREEN**: Modificar `ContainerTable.tsx`:
  - [x] Cambiar `gap: 4` a `gap: 2` en grid mobile
  - [x] Envolver contenedores sin stack en contenedor con fondo sutil
- [x] **GREEN**: Modificar `ContainerRow.tsx` (mobile branch):
  - [x] Añadir `background: var(--ant-color-fill-tertiary)` a cada fila
  - [x] Añadir `border-radius: 6px` y `margin-bottom: 2px`
  - [x] Reducir padding de 4 a 2
  - [x] Reemplazar `Card bordered` anidado por `div` simple con fondo
- [x] **GREEN**: Modificar `DashboardPage.tsx` (mobile renderGroup):
  - [x] Quitar `bordered` del Card de stack
  - [x] Reducir padding del body del Card a 0

### 4. Verificación final
- [x] Ejecutar `npm test` en frontend — ✅ 18 tests pass
- [x] Ejecutar `npx tsc --noEmit` en frontend — ✅ sin errores
- [x] Ejecutar `npm run lint` en frontend — ✅ solo warnings pre-existentes