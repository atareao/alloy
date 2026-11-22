# ui/layout Specification

## Purpose
Define la estructura de navegación de Alloy basada en topbar: el `Layout.Header`
con logo y botones de navegación en desktop y mobile, y la ausencia de `Sider`
o menú hamburguesa.

## Requirements

### Requirement: Navegación via Topbar en desktop
En desktop (>768px), la aplicación MUST renderizar la navegación en el
`Layout.Header` (topbar), con el logo + "Alloy" a la izquierda y 4 botones
(Dashboard, Historial, Config, Salir) a la derecha.
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
En mobile (≤768px), la aplicación MUST renderizar la navegación en el
`Layout.Header`, con el logo a la izquierda y 4 botones con solo icono a la
derecha.
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
La aplicación MUST NOT renderizar un `Layout.Sider` ni un botón flotante de
hamburguesa, ni mantener el estado `siderCollapsed`.
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
