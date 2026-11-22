# backend/container-pagination Specification

## Purpose
Esta capability cubre la paginación y el filtrado del listado de contenedores en el servidor: parámetros `page`/`page_size`, orden determinista, entrega de la página solicitada por tiempo real y un resumen global. Garantiza respuestas consistentes y escalables para `GET /api/containers`.

## Requirements

### Requirement: El listado SHALL paginarse en el servidor

`GET /api/containers` MUST aceptar los parámetros `page` y `page_size` y MUST responder con un objeto `{ items, total, page, page_size }`, donde `items` es la página solicitada, `total` el número de contenedores tras filtrar, y `page`/`page_size` los valores efectivos aplicados. `page` MUST ser >= 1 y por defecto 1; `page_size` MUST pertenecer a {10, 20, 50} y por defecto 20.

#### Scenario: Página por defecto

- **GIVEN** 25 contenedores y ninguna query
- **WHEN** se llama a `GET /api/containers`
- **THEN** `page` es 1, `page_size` es 20, `total` es 25 e `items` tiene 20 elementos

#### Scenario: Segunda página

- **GIVEN** 25 contenedores
- **WHEN** se llama a `GET /api/containers?page=2&page_size=20`
- **THEN** `items` tiene 5 elementos y `total` es 25

#### Scenario: Página más allá del final

- **GIVEN** 25 contenedores
- **WHEN** se llama a `GET /api/containers?page=99`
- **THEN** `items` está vacío y `total` sigue siendo 25

#### Scenario: Parámetros inválidos

- **GIVEN** un `page_size` de 30 o un `page` de 0
- **WHEN** se llama a `GET /api/containers?page_size=30`
- **THEN** la respuesta es `400`

La query MUST tolerar parámetros escalares repetidos aplicando el **último** valor: `?page=1&page=2` equivale a `?page=2` y no MUST producir `400`. Los parámetros **desconocidos** (p. ej. `pageSize=50`) MUST ignorarse, de modo que el campo correspondiente cae a su valor por defecto. `state` es el único parámetro repetible con semántica OR; el resto de parámetros repetidos siguen la regla de "gana el último".

### Requirement: El endpoint SHALL filtrar en el servidor

`GET /api/containers` MUST aceptar `q` (substring case-insensitive sobre `name`, `image` y `compose_project`), `state` (puede repetirse; se devuelven los contenedores cuyo estado esté en el conjunto indicado, con semántica OR) y `has_update` (booleano; `true` limita a contenedores con actualización pendiente). `total` MUST ser el recuento **después** de filtrar, y los filtros MUST aplicarse antes de paginar.

#### Scenario: Búsqueda por texto

- **GIVEN** contenedores "web", "db" y "cache"
- **WHEN** se llama a `GET /api/containers?q=web`
- **THEN** `total` es 1 e `items` contiene solo "web"

#### Scenario: Filtro por estado

- **GIVEN** contenedores en estado `running`, `exited` y `paused`
- **WHEN** se llama a `GET /api/containers?state=running&state=exited`
- **THEN** se devuelven los `running` y los `exited` (no los `paused`) y `total` refleja ese subconjunto

#### Scenario: Filtro por actualización pendiente

- **GIVEN** contenedores con y sin `has_update`
- **WHEN** se llama a `GET /api/containers?has_update=true`
- **THEN** solo se devuelven los que tienen `has_update` verdadero

#### Scenario: Filtros combinados con paginación

- **GIVEN** 30 contenedores `running`, de los cuales 12 tienen actualización
- **WHEN** se llama a `GET /api/containers?state=running&has_update=true&page_size=10&page=2`
- **THEN** `total` es 12 e `items` tiene 2 elementos

### Requirement: El orden SHALL ser determinista

La lista MUST ordenarse por `compose_project` ascendente y, dentro de cada proyecto, por `name` ascendente, antes de paginar, de modo que dos peticiones con los mismos parámetros devuelvan el mismo orden.

#### Scenario: Orden estable

- **GIVEN** contenedores de los proyectos "alpha" y "beta" mezclados
- **WHEN** se llama dos veces a `GET /api/containers?page=1&page_size=10`
- **THEN** ambas respuestas devuelven los mismos `items` en el mismo orden

#### Scenario: Proyectos contiguos

- **GIVEN** contenedores de "alpha" (2) y "beta" (2)
- **WHEN** se pide la página completa
- **THEN** los de "alpha" aparecen juntos y antes que los de "beta"

### Requirement: El tiempo real SHALL entregar la página solicitada

`/api/state` y `/api/stream` MUST aceptar los mismos parámetros de paginación y filtrado que `GET /api/containers`. El `snapshot` del SSE y la respuesta del long-poll MUST contener en `containers` solo la página solicitada, y `StateResponse` MUST exponer `total`, `page` y `page_size`.

#### Scenario: Snapshot paginado

- **GIVEN** 25 contenedores y una conexión a `/api/stream?page=1&page_size=10`
- **WHEN** se recibe el primer evento `snapshot`
- **THEN** `containers` tiene 10 elementos, `total` es 25 y `page_size` es 10

#### Scenario: Filtro en el snapshot

- **GIVEN** contenedores con y sin actualización
- **WHEN** se conecta a `/api/stream?has_update=true`
- **THEN** el `snapshot` solo incluye contenedores con `has_update`

#### Scenario: Long-poll coherente con el REST

- **GIVEN** los mismos parámetros
- **WHEN** se llama a `/api/state` y a `/api/containers`
- **THEN** ambos devuelven los mismos `items` y `total`

### Requirement: El resumen SHALL seguir siendo global

El `summary` de `StateResponse` MUST calcularse sobre **todos** los contenedores de la flota, con independencia de la página y los filtros solicitados.

#### Scenario: Resumen global con filtro activo

- **GIVEN** 25 contenedores de los que 3 están parados
- **WHEN** se pide `GET /api/stream?state=running`
- **THEN** el `snapshot` incluye solo contenedores `running` pero `summary.total` es 25

### Requirement: El backend SHALL seguir verde

Tras el cambio, `cargo test` y `just check` MUST terminar con código 0.

#### Scenario: Verificación backend

- **WHEN** se ejecutan `cargo test` y `just check`
- **THEN** ambos terminan con código 0
