# Spec Delta: backend/container-pagination — ui-pagination-and-vertical-tabs

## ADDED Requirements

### Requirement: El listado SHALL poder solicitarse completo

`GET /api/containers`, `/api/state` y `/api/stream` MUST aceptar el parámetro booleano `all`. Cuando `all=true`, la respuesta MUST contener TODOS los contenedores tras aplicar los filtros, en el orden determinista ya especificado; `total` MUST ser el número de contenedores filtrados e `items`/`containers` MUST incluir todos ellos. La respuesta MUST exponer `page=1` y `page_size` igual al número de items devueltos. Cuando `all=true`, los parámetros `page` y `page_size` de la petición MUST ignorarse y MUST NOT validarse su rango. Cuando `all` está ausente o es `false`, el comportamiento paginado actual MUST permanecer inalterado.

#### Scenario: Flota completa

- **GIVEN** 120 contenedores
- **WHEN** se llama a `GET /api/containers?all=true`
- **THEN** `items` tiene 120 elementos
- **AND** `total` es 120

#### Scenario: Filtro con all=true

- **GIVEN** contenedores en estado `running` y `exited`
- **WHEN** se llama a `GET /api/containers?all=true&state=running`
- **THEN** `items` contiene todos los `running`
- **AND** `total` es el número de `running`

#### Scenario: Snapshot de tiempo real completo

- **GIVEN** una conexión a `/api/stream?all=true`
- **WHEN** se recibe el primer evento `snapshot`
- **THEN** `containers` contiene todos los contenedores filtrados
- **AND** `total` es su número

#### Scenario: all ausente mantiene la paginación

- **GIVEN** una petición sin `all`
- **WHEN** se llama a `GET /api/containers`
- **THEN** `page` es 1, `page_size` es 20
- **AND** `GET /api/containers?page_size=30` sigue devolviendo `400`

#### Scenario: all=true ignora page_size inválido

- **GIVEN** la flota completa
- **WHEN** se llama a `GET /api/containers?all=true&page_size=30`
- **THEN** la respuesta es `200` con todos los items
