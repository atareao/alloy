# frontend/container-list Specification

## Purpose
TBD - created by archiving change container-row-ellipsis. Update Purpose after archive.

## Requirements

### Requirement: El nombre y el estado SHALL truncarse con elipsis real

La fila del contenedor MUST truncar el nombre y el estado con elipsis real (CSS/Ant Design) cuando no quepan, en las variantes móvil y de escritorio. MUST NOT cortar el texto manualmente con `slice` ni añadir `"..."` a mano.

#### Scenario: Nombre largo en móvil

- **GIVEN** un contenedor cuyo nombre no cabe en el ancho de la tarjeta
- **WHEN** se renderiza la fila en la variante móvil
- **THEN** el nombre se muestra con elipsis real, sin `slice` manual
- **AND** el texto completo está disponible en un tooltip

#### Scenario: Nombre largo en escritorio

- **GIVEN** un contenedor cuyo nombre no cabe en el ancho de la fila
- **WHEN** se renderiza la fila en la variante de escritorio
- **THEN** el nombre se trunca con elipsis real
- **AND** el texto completo está disponible en un tooltip

#### Scenario: Estado largo

- **GIVEN** un contenedor cuyo estado no cabe en el ancho disponible
- **WHEN** se renderiza la fila
- **THEN** el estado se trunca con elipsis real
- **AND** el texto completo está disponible en un tooltip

### Requirement: El truncado SHALL ser efectivo en el layout

Los contenedores flex de nombre y estado MUST declarar `minWidth: 0` (y permitir encogerse) para que la elipsis se aplique de verdad. Los elementos no truncables de la fila (badge de actualización, spinner, enlace de Traefik, botón de expansión) MUST NOT comprimirse ni desaparecer.

#### Scenario: Elementos no truncables intactos

- **GIVEN** un contenedor con nombre muy largo y badge de actualización
- **WHEN** se renderiza la fila
- **THEN** el badge, el spinner y el botón de expansión siguen visibles
- **AND** el nombre es el que se trunca

### Requirement: El texto de progreso SHALL truncarse con elipsis

El texto `progress.checking` de la fila MUST truncarse con elipsis real en lugar de `slice(0, 20)`, y MUST mostrar el valor completo en el tooltip existente.

#### Scenario: Progreso largo

- **GIVEN** un `progress.checking` más largo que el espacio disponible
- **WHEN** se renderiza la fila durante una comprobación
- **THEN** el texto se trunca con elipsis real
- **AND** el tooltip muestra el valor completo

### Requirement: La imagen del contenedor SHALL quedar fuera de la fila

La fila MUST NOT mostrar la imagen del contenedor en este change. La imagen se muestra en `InspectModal` (`containerInfo.image`); mostrarla en la fila requeriría un change incremental.

#### Scenario: Sin imagen en la fila

- **GIVEN** un contenedor
- **WHEN** se renderiza la fila
- **THEN** no se renderiza la imagen ni su etiqueta

### Requirement: La suite SHALL seguir verde

Tras el cambio, `pnpm run test`, `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` MUST terminar con código 0 en `frontend/`, y el recuento de tests MUST ser mayor o igual a 174.

#### Scenario: Verificación frontend

- **WHEN** se ejecutan los comandos de verificación
- **THEN** todos terminan con código 0
- **AND** hay al menos 174 tests
