## ADDED Requirements

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
