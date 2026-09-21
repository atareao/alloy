# UI Layout — Sistema de layout responsivo con Sidebar

## ADDED Requirements

### Requirement: Layout sin marco/borde
**Given** un usuario autenticado en Alloy
**When** se renderiza la aplicación
**Then** el `Layout` principal tiene `background: transparent`
**And** no hay bordes visibles alrededor del layout
**And** el `Layout.Header` solo tiene `borderBottom` sutil

#### Scenario: Layout sin marco/borde
**Given** un usuario autenticado en Alloy
**When** se renderiza la aplicación
**Then** el `Layout` principal tiene `background: transparent`
**And** no hay bordes visibles alrededor del layout
**And** el `Layout.Header` solo tiene `borderBottom` sutil

### Requirement: Navegación via Sidebar en desktop
**Given** un usuario en desktop (>768px)
**When** se renderiza la aplicación
**Then** el `Layout.Sider` es visible a la izquierda
**And** el `Menu` muestra los items Dashboard, Historial, Config, Salir con iconos y texto
**And** el Sider es colapsable (toggle con hamburguesa)
**And** al hacer clic en un item del menú, cambia la vista

#### Scenario: Navegación via Sidebar en desktop
**Given** un usuario en desktop (>768px)
**When** se renderiza la aplicación
**Then** el `Layout.Sider` es visible a la izquierda
**And** el `Menu` muestra los items Dashboard, Historial, Config, Salir con iconos y texto
**And** el Sider es colapsable
**And** al hacer clic en un item del menú, cambia la vista

### Requirement: Navegación via Sidebar en mobile
**Given** un usuario en mobile (≤768px)
**When** se renderiza la aplicación
**Then** el `Layout.Sider` se muestra como drawer overlay
**And** el Sider se abre/cierra con el trigger (hamburguesa)
**And** al seleccionar un item, el drawer se cierra automáticamente

#### Scenario: Navegación via Sidebar en mobile
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

#### Scenario: Header limpio sin navegación
**Given** un usuario autenticado
**When** se renderiza el `Layout.Header`
**Then** solo contiene el logo de Alloy y el nombre del usuario
**And** no contiene botones de navegación