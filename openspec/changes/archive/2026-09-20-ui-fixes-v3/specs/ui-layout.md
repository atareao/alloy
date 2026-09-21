# ui/layout Specification

## Requirements

### Requirement: Cards de stack con borde visible en mobile
**Given** un usuario en mobile (≤768px)
**When** se renderiza el grid de stacks en el dashboard
**Then** cada stack se muestra como una Card con borde visible (estilo Ant Design por defecto)
**And** la Card tiene `background: var(--ant-color-bg-container)` (por defecto)
**And** la Card tiene `borderRadius` y `border` estándar de Ant Design

#### Scenario: Stack card con borde en mobile
**Given** un usuario en mobile
**When** se renderiza un stack en el grid
**Then** la Card NO tiene `bordered={false}`
**And** la Card tiene el borde por defecto de Ant Design (1px solid)
**And** la Card se ve como una tarjeta independiente

### Requirement: Grid sin background propio
**Given** un usuario en mobile
**When** se renderiza el grid de stacks
**Then** el contenedor grid tiene `background: transparent`
**And** la separación entre cards viene de sus propios bordes y del `gap` del grid

#### Scenario: Grid con background transparente
**Given** un usuario en mobile
**When** se renderiza el grid
**Then** el `background` del grid es `transparent`
**And** no hay líneas de rejilla adicionales compitiendo con los bordes de las cards