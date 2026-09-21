# ui/layout Specification

## Requirements

### Requirement: Cards de stack con fondo contrastado en mobile
**Given** un usuario en mobile (≤768px)
**When** se renderiza el grid de stacks en el dashboard
**Then** cada stack se muestra como una Card con fondo `var(--ant-color-bg-container)` (por defecto de Ant Design)
**And** el fondo de la Card es **distinto** del fondo de página (`var(--ant-color-bg-layout)`)
**And** la Card tiene borde visible, border-radius y padding interior
**And** hay separación visible (≥8px) entre cada Card

#### Scenario: Stack card sin override de background
**Given** un usuario en mobile
**When** se renderiza un stack en el grid
**Then** el inner div del Card NO tiene `background: var(--ant-color-bg-layout)`
**And** el fondo visible de la Card es el nativo de Ant Design (`bg-container`)
**And** la Card se ve como una tarjeta independiente, similar a las cards de stats (Total, Running, Stopped)

#### Scenario: Card con esquinas redondeadas visibles
**Given** un usuario en mobile
**When** se renderiza un stack en el grid
**Then** la Card muestra sus esquinas redondeadas (border-radius nativo de Ant Design)
**And** el borde de la Card es visible alrededor de todo el perímetro

#### Scenario: Separación entre cards
**Given** un usuario en mobile
**When** se renderiza el grid de stacks
**Then** el contenedor grid tiene `gap: 8` (o mayor)
**And** hay espacio visible entre cada Card del grid