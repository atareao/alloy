# Spec Delta: ui/layout

## REMOVED Requirements

### Requirement: Navegación via Sidebar en desktop
**Given** un usuario en desktop (>768px)
**When** se renderiza la aplicación
**Then** el `Layout.Sider` es visible a la izquierda
**And** el `Menu` muestra los items Dashboard, Historial, Config, Salir con iconos y texto
**And** el Sider es colapsable (toggle con hamburguesa)
**And** al hacer clic en un item del menú, cambia la vista

### Requirement: Navegación via Sidebar en mobile
**Given** un usuario en mobile (≤768px)
**When** se renderiza la aplicación
**Then** el `Layout.Sider` se muestra como drawer overlay
**And** el Sider se abre/cierra con el trigger (hamburguesa)
**And** al seleccionar un item, el drawer se cierra automáticamente

### Requirement: Header limpio sin navegación
**Given** un usuario autenticado
**When** se renderiza el `Layout.Header`
**Then** solo contiene el logo de Alloy y el nombre del usuario
**And** no contiene botones de navegación

## ADDED Requirements

### Requirement: Navegación via Topbar en desktop
**Given** un usuario en desktop (>768px)
**When** se renderiza la aplicación
**Then** el `Layout.Header` contiene el logo + "Alloy" a la izquierda
**And** contiene 4 botones de navegación a la derecha: Dashboard, Historial, Config, Salir
**And** cada botón muestra icono + texto
**And** al hacer clic en un botón, cambia la vista (excepto Salir que cierra sesión)
**And** el botón de la vista activa está destacado visualmente

#### Scenario: Navegación via Topbar en desktop
**Given** un usuario en desktop (>768px)
**When** se renderiza la aplicación
**Then** el `Layout.Header` tiene logo + "Alloy" a la izquierda
**And** los botones Dashboard, Historial, Config, Salir están a la derecha con icono+texto
**When** el usuario hace clic en "Historial"
**Then** la vista cambia a historial
**And** el botón "Historial" se muestra como activo

### Requirement: Navegación via Topbar en mobile
**Given** un usuario en mobile (≤768px)
**When** se renderiza la aplicación
**Then** el `Layout.Header` contiene el logo a la izquierda
**And** contiene 4 botones de navegación a la derecha con solo icono (sin texto)
**And** al hacer clic en un botón, cambia la vista (excepto Salir que cierra sesión)

#### Scenario: Navegación via Topbar en mobile
**Given** un usuario en mobile (≤768px)
**When** se renderiza la aplicación
**Then** el `Layout.Header` tiene logo a la izquierda
**And** los botones tienen solo iconos (📊📋⚙️🚪) sin texto visible
**When** el usuario hace clic en el icono de configuración
**Then** la vista cambia a configuración

### Requirement: Sin Sider ni hamburguesa
**Given** un usuario autenticado
**When** se renderiza la aplicación
**Then** NO hay `Layout.Sider` en el DOM
**And** NO hay botón flotante de hamburguesa
**And** NO hay estado `siderCollapsed`

#### Scenario: Sin Sider ni hamburguesa
**Given** un usuario autenticado
**When** se renderiza la aplicación
**Then** no existe `Layout.Sider`
**And** no existe botón `MenuUnfoldOutlined`