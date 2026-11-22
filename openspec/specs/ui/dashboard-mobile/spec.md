# ui/dashboard-mobile Specification

## Purpose
Define el layout responsive del Dashboard en viewports mobile (≤768px):
filas de contenedores compactas y visualmente distinguibles del fondo, grupos
de stack y grid sin bordes ni padding extra de Card, y paneles de acciones
expandidos sin Card anidado.

## Requirements

### Requirement: Filas compactas sin espacios excesivos
Las filas de contenedores en mobile MUST mostrarse compactas, sin espacios
verticales excesivos entre filas consecutivas.
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
Cada fila de contenedor en mobile MUST distinguirse visualmente del fondo de
página.
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
El Card contenedor de un grupo de stack en mobile MUST NOT tener `bordered` y
MUST usar un padding interno mínimo.
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
El grid de stacks en mobile MUST usar un gap de 2px.
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza el grid de stacks
**Then** el gap del grid es de 2px (no 4px)

#### Scenario: Grid de stacks compacto
**Given** un usuario en mobile en la vista Dashboard
**When** se renderiza el grid de stacks
**Then** el gap del grid es de 2px

### Requirement: Panel expandido sin Card anidado
El panel de acciones expandido en mobile MUST NOT estar envuelto en un
`Card bordered`; MUST usar un `div` simple con fondo y `border-radius`.
**Given** un usuario en mobile expande una fila de contenedor
**When** se renderiza el panel de acciones expandido
**Then** el panel no está envuelto en un `Card bordered`
**And** el panel usa un `div` simple con fondo y `border-radius`

#### Scenario: Panel expandido sin Card anidado
**Given** un usuario en mobile expande una fila de contenedor
**When** se renderiza el panel de acciones expandido
**Then** el panel no está envuelto en un `Card bordered`
**And** el panel usa un `div` simple con fondo y `border-radius`
