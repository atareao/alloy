# Dashboard Mobile — Vista Dashboard optimizada para mobile

## ADDED Requirements

### Requirement: Filas compactas sin espacios excesivos
**Given** un usuario en mobile (≤768px) en la vista Dashboard
**When** se renderizan las filas de contenedores
**Then** no hay espacios verticales grandes entre filas consecutivas
**And** el gap entre filas es de máximo 2px
**And** las filas no están envueltas en Cards con bordes que añadan padding extra

#### Scenario: Filas compactas sin espacios excesivos
**Given** un usuario en mobile (≤768px) en la vista Dashboard
**When** se renderizan las filas de contenedores
**Then** no hay espacios verticales grandes entre filas consecutivas
**And** el gap entre filas es de máximo 2px
**And** las filas no están envueltas en Cards con bordes que añadan padding extra

### Requirement: Filas distinguibles del fondo
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza una fila de contenedor
**Then** la fila tiene un fondo diferente al fondo de página
**And** la fila tiene `border-radius` sutil (4-6px) para parecer una tarjeta
**And** hay una separación visual clara entre filas

#### Scenario: Filas distinguibles del fondo
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza una fila de contenedor
**Then** la fila tiene un fondo diferente al fondo de página
**And** la fila tiene `border-radius` sutil (4-6px)
**And** hay una separación visual clara entre filas

### Requirement: Stack groups en mobile sin bordes de Card
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza un grupo de stack
**Then** el Card contenedor del stack no tiene `bordered`
**And** el padding interno del Card es mínimo (0-4px)

#### Scenario: Stack groups en mobile sin bordes de Card
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza un grupo de stack
**Then** el Card contenedor del stack no tiene `bordered`
**And** el padding interno del Card es mínimo (0-4px)

### Requirement: Grid de stacks compacto
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza el grid de stacks
**Then** el gap del grid es de 2px (no 4px)

#### Scenario: Grid de stacks compacto
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza el grid de stacks
**Then** el gap del grid es de 2px

### Requirement: Panel expandido sin Card anidado
**Given** un usuario en mobile expande una fila de contenedor
**When** se renderiza el panel de acciones expandido
**Then** el panel no está envuelto en un `Card bordered`
**And** el panel usa un `div` simple con fondo y `border-radius`

#### Scenario: Panel expandido sin Card anidado
**Given** un usuario en mobile expande una fila de contenedor
**When** se renderiza el panel de acciones expandido
**Then** el panel no está envuelto en un `Card bordered`
**And** el panel usa un `div` simple con fondo y `border-radius`