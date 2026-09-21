# ui/layout Specification

## Requirements

### Requirement: Celdas de grid mobile sin huecos
**Given** un usuario en mobile (≤768px) con stacks de contenedores
**When** se renderiza el grid de stacks en el dashboard
**Then** cada celda del grid tiene `aspectRatio: "1"` (cuadrado)
**And** la `Card` dentro de cada celda tiene `height: 100%` para llenar todo el cuadrado
**And** no hay espacio vacío sobrante entre la Card y el borde de la celda

#### Scenario: Card llena toda la celda cuadrada
**Given** un usuario en mobile
**When** se renderiza una celda de stack no expandida
**Then** la `Card` tiene `height: 100%`
**And** el contenido de la Card llena todo el espacio disponible
**And** no hay hueco visible entre la Card y el borde de la celda

### Requirement: Rejilla visible entre celdas
**Given** un usuario en mobile (≤768px) con stacks de contenedores
**When** se renderiza el grid de stacks
**Then** el contenedor grid tiene `background: var(--ant-color-border-secondary)`
**And** el `gap: 1px` crea líneas de rejilla visibles entre celdas
**And** cada celda se distingue claramente de sus vecinas

#### Scenario: Grid mobile con separación visible entre celdas
**Given** un usuario en mobile
**When** se renderiza el grid de stacks
**Then** el contenedor grid tiene `background: var(--ant-color-border-secondary)`
**And** el `gap: 1px` crea líneas de rejilla visibles

#### Scenario: Tema oscuro mantiene separación visible
**Given** un usuario en mobile con tema oscuro
**When** se renderiza el grid de stacks
**Then** `var(--ant-color-border-secondary)` se resuelve a un color visible
**And** las líneas de rejilla de 1px son perceptibles

#### Scenario: Tema claro mantiene separación visible
**Given** un usuario en mobile con tema claro
**When** se renderiza el grid de stacks
**Then** `var(--ant-color-border-secondary)` se resuelve a un color visible
**And** las líneas de rejilla de 1px son perceptibles