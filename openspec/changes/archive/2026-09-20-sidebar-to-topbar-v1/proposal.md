# Proposal: Sidebar a Topbar

## Why

El sidebar vertical ocupa espacio horizontal valioso, especialmente en desktop donde el contenido principal necesita más anchura. En mobile, el sidebar como drawer overlay añade complejidad (botón hamburguesa flotante, animación de apertura/cierre). Una barra superior fija con navegación horizontal resuelve ambos problemas:

1. **Desktop**: La navegación está siempre visible sin ocupar espacio lateral. Todo el ancho disponible es para el contenido.
2. **Mobile**: Los botones de navegación con iconos están siempre accesibles sin necesidad de abrir/cerrar un drawer. El logo queda a la izquierda, los iconos a la derecha.

## What Changes

### 1. Eliminar Layout.Sider
- Eliminar el `Layout.Sider` completo de `App.tsx`
- Eliminar el `Menu` vertical con items Dashboard, Historial, Config, Salir
- Eliminar el botón flotante de hamburguesa para mobile
- Eliminar el estado `siderCollapsed`

### 2. Transformar Layout.Header en topbar con navegación
- Mover los 4 items de navegación (Dashboard, Historial, Config, Salir) al `Layout.Header`
- En desktop: logo + nombre a la izquierda, botones con icono+texto a la derecha
- En mobile: logo a la izquierda, botones con solo icono a la derecha
- Usar `Button` de Ant Design en vez de `Menu` items, con estilo inline

### 3. Ajustar layout del contenido
- El `Layout.Content` ya no necesita `marginLeft` para compensar el Sider
- El header ahora es la única navegación

## Capabilities

### MODIFIED: `ui/layout`
- **Antes**: Navegación via `Layout.Sider` vertical colapsable + `Layout.Header` sin navegación
- **Después**: Navegación via `Layout.Header` con logo+nombre a izquierda y botones a derecha. Sin Sider.

## Impact

- **Archivos afectados**: `frontend/src/App.tsx`
- **Sin cambios de API**: Solo UI
- **Sin cambios de dependencias**: Se elimina `Menu` y `MenuUnfoldOutlined` de imports
- **Sin cambios de tests**: Los tests existentes no verifican la estructura del layout