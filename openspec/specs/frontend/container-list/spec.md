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

### Requirement: Los filtros SHALL resolverse en el servidor

Los filtros de búsqueda, estado y pendientes de actualización MUST enviarse al servidor como parámetros de `GET /api/containers` y del stream. El cliente MUST NOT filtrar la lista localmente para decidir qué filas se muestran.

#### Scenario: La búsqueda llega al servidor

- **GIVEN** el dashboard cargado
- **WHEN** el usuario escribe "web" en el buscador
- **THEN** el cliente pide `/api/containers` con `q=web`
- **AND** la tabla muestra los `items` devueltos por el servidor

#### Scenario: El filtro de estado llega al servidor

- **GIVEN** el dashboard cargado
- **WHEN** el usuario selecciona los estados `running` y `exited` en el filtro de estado
- **THEN** el cliente pide `/api/containers` con los estados seleccionados como parámetros `state` repetidos (`?state=running&state=exited`)

### Requirement: El listado SHALL paginarse en el servidor

La tabla MUST solicitar una única página al servidor y MUST NOT recibir la flota completa. La agrupación por stack MUST hacerse sobre los `items` de la página.

#### Scenario: Una sola página en memoria

- **GIVEN** 25 contenedores y un tamaño de página de 10
- **WHEN** el dashboard carga
- **THEN** el cliente pide `/api/containers?page=1&page_size=10`
- **AND** solo se renderizan 10 contenedores

### Requirement: El paginador SHALL ofrecer 10/20/50

La tabla MUST mostrar un `Pagination` con el total recibido, el tamaño de página actual y un selector con las opciones 10, 20 y 50. Al cambiar de página o de tamaño, el cliente MUST volver a consultar el servidor.

#### Scenario: Cambio de página

- **GIVEN** 25 contenedores con tamaño 10
- **WHEN** el usuario pulsa la página 2
- **THEN** el cliente pide `/api/containers?page=2&page_size=10`

#### Scenario: Cambio de tamaño de página

- **GIVEN** el dashboard con tamaño 10
- **WHEN** el usuario elige 50
- **THEN** el cliente pide `/api/containers?page=1&page_size=50`

#### Scenario: El paginador refleja el total filtrado

- **GIVEN** un filtro que deja 12 contenedores
- **WHEN** se muestra el paginador
- **THEN** el total mostrado es 12

### Requirement: El tiempo real SHALL seguir la página y los filtros

La conexión de tiempo real (`EventSource` de `/api/stream`, con el long-poll de `/api/state` como respaldo) MUST usar los mismos parámetros que el REST y MUST reconectarse cuando cambien la página, el tamaño o los filtros.

#### Scenario: Reconexión al cambiar de página

- **GIVEN** una conexión activa a `/api/stream?page=1&page_size=20`
- **WHEN** el usuario cambia a la página 2
- **THEN** la conexión se reabre con `page=2`

#### Scenario: El snapshot actualiza la página visible

- **GIVEN** una conexión con filtros activos
- **WHEN** llega un evento `snapshot`
- **THEN** la tabla se actualiza con los `items` y el `total` del snapshot

### Requirement: La búsqueda SHALL estar debounced

La búsqueda por texto MUST debouncearse antes de construir la query, de modo que no se emita una petición por cada pulsación de tecla.

#### Scenario: Varias pulsaciones, una petición

- **GIVEN** el buscador vacío
- **WHEN** el usuario escribe "web" tecla a tecla
- **THEN** no se lanza una petición por cada tecla

### Requirement: La expansión de filas SHALL reiniciarse al cambiar de página

Al cambiar de página, tamaño o filtros, las filas y los stacks expandidos MUST colapsarse, para no arrastrar estado de contenedores que ya no están visibles.

#### Scenario: Colapso al paginar

- **GIVEN** una fila expandida en la página 1
- **WHEN** el usuario pasa a la página 2
- **THEN** no queda ninguna fila expandida

### Requirement: El progreso de check-all SHALL usar el total global

El total del progreso de `check-all` MUST tomarse del total global de la flota (el `summary`), no del número de contenedores de la página.

#### Scenario: Progreso con página parcial

- **GIVEN** 25 contenedores y una página de 10
- **WHEN** se lanza un `check-all`
- **THEN** el total del progreso es 25

### Requirement: La suite SHALL seguir verde tras la paginación

Tras el cambio, `pnpm run test`, `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` MUST terminar con código 0 en `frontend/`.

#### Scenario: Verificación frontend

- **WHEN** se ejecutan los comandos de verificación
- **THEN** todos terminan con código 0
