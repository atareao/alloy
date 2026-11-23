# Spec Delta: frontend/container-list — ui-pagination-and-vertical-tabs

## ADDED Requirements

### Requirement: El listado SHALL mostrarse completo sin paginación

La tabla del Dashboard MUST mostrar todos los contenedores filtrados en una sola vista y MUST NOT renderizar ningún control `Pagination` ni selector de tamaño de página. El cliente MUST solicitar la flota completa con `all=true`. La agrupación por stack MUST hacerse sobre el conjunto completo.

#### Scenario: Todos los contenedores visibles

- **GIVEN** 37 contenedores
- **WHEN** el dashboard carga
- **THEN** el cliente pide `/api/containers?...&all=true`
- **AND** se renderizan las 37 filas
- **AND** no existe ningún componente `Pagination`

#### Scenario: Grupos de stack completos

- **GIVEN** un grupo con un contenedor y sus réplicas
- **WHEN** el dashboard carga
- **THEN** el grupo se muestra completo, sin recortes por paginación

### Requirement: El tiempo real SHALL seguir los filtros

La conexión de tiempo real (`EventSource` de `/api/stream`, con el long-poll de `/api/state` como respaldo) MUST usar los mismos parámetros que el REST, incluido `all=true`, y MUST reconectarse cuando cambien los filtros.

#### Scenario: Reconexión al cambiar un filtro

- **GIVEN** una conexión activa a `/api/stream?all=true`
- **WHEN** el usuario cambia el filtro de estado
- **THEN** la conexión se reabre con el nuevo filtro y `all=true`

### Requirement: La expansión de filas SHALL reiniciarse al cambiar los filtros

Al cambiar los filtros, las filas y los stacks expandidos MUST colapsarse, para no arrastrar estado de contenedores que ya no están visibles.

#### Scenario: Colapso al filtrar

- **GIVEN** una fila expandida
- **WHEN** el usuario cambia el filtro
- **THEN** no queda ninguna fila expandida

## REMOVED Requirements

### Requirement: El listado SHALL paginarse en el servidor

El Dashboard ya no solicita una única página: solicita la flota completa con `all=true` y la renderiza entera.

### Requirement: El paginador SHALL ofrecer 10/20/50

Se elimina por completo el control de paginación del Dashboard.

### Requirement: El tiempo real SHALL seguir la página y los filtros

El Dashboard deja de tener página; la conexión de tiempo real solo sigue los filtros.

### Requirement: La expansión de filas SHALL reiniciarse al cambiar de página

Reemplazado por "La expansión de filas SHALL reiniciarse al cambiar los filtros".
