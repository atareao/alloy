# Proposal

## Why

La interfaz de Alloy tiene tres problemas visuales que afectan la experiencia de usuario: (1) un marco/borde visible alrededor de toda la aplicación en ambos temas (oscuro y claro), (2) las pestañas de navegación (Dashboard, Historial, Config, Salir) no caben en la ventana y se salen del layout, y (3) en la vista Dashboard en modo mobile, las filas de contenedores tienen espacios enormes entre sí y no se distinguen visualmente del fondo de página.

## What Changes

1. **Eliminar marco/borde alrededor de la app**: Ajustar el `background` del `Layout` principal y eliminar bordes no deseados que crean un efecto de marco.
2. **Reemplazar tabs por Sidebar**: Sustituir los botones de navegación en el `Layout.Header` por un `Layout.Sider` de Ant Design con `Menu` colapsable, que funciona correctamente en desktop y mobile.
3. **Compactar Dashboard mobile**: Reducir gaps entre filas de contenedores en mobile, añadir fondos diferenciadores a cada fila, y eliminar bordes/Cards innecesarios que crean espacios verticales excesivos.

## Capabilities

### New Capabilities
- `ui/layout`: Sistema de layout responsivo con Sidebar de navegación, sin bordes/marcos no deseados, que se adapta correctamente a desktop y mobile.
- `ui/dashboard-mobile`: Vista Dashboard optimizada para mobile con filas compactas, sin espacios excesivos entre filas, y con diferenciación visual entre filas y fondo.

### Modified Capabilities
- Ninguna — no hay specs existentes en el proyecto.

## Impact

- **Archivos modificados**:
  - `frontend/src/App.tsx` — Reemplazar navegación por botones con `Layout.Sider` + `Menu`
  - `frontend/src/components/ContainerTable.tsx` — Reducir gaps en grid mobile, simplificar Cards
  - `frontend/src/components/ContainerRow.tsx` — Añadir fondo a filas mobile, quitar Card anidado, reducir padding
  - `frontend/src/components/DashboardPage.tsx` — Quitar `bordered` de Cards stack en mobile, reducir padding
- **Solo frontend**: No hay cambios en backend, API, dependencias ni sistemas.