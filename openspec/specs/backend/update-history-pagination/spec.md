# backend/update-history-pagination Specification

## Purpose
Definir el contrato paginado y filtrado de `GET /api/history`, servido sobre el histórico de actualizaciones persistido, de forma consistente con la paginación del listado de contenedores.

## Requirements

### Requirement: El histórico SHALL servirse paginado con envelope

`GET /api/history` MUST aceptar los parámetros `page` y `page_size` y MUST responder con un objeto `{ items, total, page, page_size }`, donde `items` es la página solicitada, `total` el número de entradas tras aplicar los filtros, y `page`/`page_size` los valores efectivos aplicados. `page` MUST ser >= 1 y por defecto 1; `page_size` MUST pertenecer a {10, 20, 50} y por defecto 20.

#### Scenario: Página por defecto

- **GIVEN** 25 entradas de histórico
- **WHEN** se llama a `GET /api/history`
- **THEN** `page` es 1, `page_size` es 20, `total` es 25 e `items` tiene 20 elementos

#### Scenario: Segunda página

- **GIVEN** 25 entradas de histórico
- **WHEN** se llama a `GET /api/history?page=2&page_size=20`
- **THEN** `items` tiene 5 elementos y `total` es 25

#### Scenario: Página más allá del final

- **GIVEN** 25 entradas de histórico
- **WHEN** se llama a `GET /api/history?page=99`
- **THEN** `items` está vacío y `total` sigue siendo 25

#### Scenario: Parámetros inválidos

- **GIVEN** un `page_size` de 30 o un `page` de 0
- **WHEN** se llama a `GET /api/history?page_size=30`
- **THEN** la respuesta es `400`

#### Scenario: Escalares repetidos

- **GIVEN** `GET /api/history?page=1&page=2`
- **WHEN** se procesa la query
- **THEN** se aplica `page=2` y la respuesta no es `400`

#### Scenario: Parámetro desconocido

- **GIVEN** `GET /api/history?pageSize=50`
- **WHEN** se procesa la query
- **THEN** el parámetro desconocido se ignora y `page_size` toma su valor por defecto de 20

### Requirement: La paginación SHALL cubrir el histórico persistido completo

`items` y `total` MUST derivarse del histórico persistido, no de un recorte previo: `total` MUST ser el número de entradas almacenadas que cumplen los filtros, sin límite superior artificial, e `items` MUST corresponder a la ventana solicitada sobre ese conjunto. La respuesta MUST NOT truncar el resultado a un número fijo de entradas.

#### Scenario: Histórico mayor que el antiguo cap de 100

- **GIVEN** 250 entradas de histórico persistidas
- **WHEN** se llama a `GET /api/history?page=13&page_size=20`
- **THEN** `total` es 250 e `items` tiene 10 elementos

#### Scenario: Total independiente de la página

- **GIVEN** 250 entradas de histórico persistidas
- **WHEN** se llama a `GET /api/history?page=2&page_size=50`
- **THEN** `total` es 250 e `items` tiene 50 elementos

### Requirement: El orden SHALL ser determinista

Las entradas MUST ordenarse por `timestamp` descendente y, ante timestamps iguales, por orden de inserción descendente, de forma que la paginación sea estable y no repita ni omita entradas entre páginas consecutivas.

#### Scenario: Timestamps iguales

- **GIVEN** tres entradas insertadas en el mismo segundo
- **WHEN** se recorren las páginas de 10 en 10
- **THEN** cada entrada aparece exactamente una vez y en el mismo orden en llamadas sucesivas

#### Scenario: Entradas más recientes primero

- **GIVEN** entradas con timestamps crecientes
- **WHEN** se llama a `GET /api/history`
- **THEN** la primera entrada de `items` es la de timestamp más reciente

### Requirement: El endpoint SHALL filtrar en el servidor

`GET /api/history` MUST aceptar `q` (substring case-insensitive sobre `container`, `image`, `old_digest` y `new_digest`) y `status` (puede repetirse; se devuelven las entradas cuyo `status` coincida con alguno de los valores indicados, con semántica OR). `total` MUST ser el recuento **después** de filtrar y los filtros MUST aplicarse antes de paginar.

#### Scenario: Búsqueda por texto

- **GIVEN** entradas de los contenedores "web", "db" y "cache"
- **WHEN** se llama a `GET /api/history?q=web`
- **THEN** `total` es 1 e `items` contiene sólo la entrada de "web"

#### Scenario: Búsqueda insensible a mayúsculas

- **GIVEN** una entrada del contenedor "Web"
- **WHEN** se llama a `GET /api/history?q=WEB`
- **THEN** `total` es 1

#### Scenario: Búsqueda sobre el digest

- **GIVEN** una entrada cuyo `new_digest` empieza por `sha256:abc`
- **WHEN** se llama a `GET /api/history?q=sha256:abc`
- **THEN** `total` es 1

#### Scenario: Filtro por estado repetible

- **GIVEN** entradas con estado `success`, `error` y `rolled-back`
- **WHEN** se llama a `GET /api/history?status=error&status=rolled-back`
- **THEN** `total` es 2 y `items` no contiene la entrada `success`

#### Scenario: Filtros combinados

- **GIVEN** entradas de "web" con estados `success` y `error`, y de "db" con `error`
- **WHEN** se llama a `GET /api/history?q=web&status=error`
- **THEN** `total` es 1

### Requirement: La lectura SHALL NOT retener el lock del histórico en memoria

El handler de lectura del histórico MUST NOT retener el lock del vector de histórico en memoria mientras espera una conexión de la base de datos, de modo que una pool saturada no pueda bloquear a los workers que publican entradas.

#### Scenario: Pool saturada

- **GIVEN** una pool con una única conexión ocupada
- **WHEN** un cliente llama a `GET /api/history`
- **THEN** el lock del histórico en memoria está libre y los `append` de los workers pueden completarse

### Requirement: El borrado SHALL seguir limpiando memoria y persistencia

`DELETE /api/history` MUST mantener su comportamiento: vaciar el histórico en memoria y borrar las filas persistidas, respondiendo `{"status":"cleared"}`. Tras el borrado, `GET /api/history` MUST responder `total` 0 e `items` vacío.

#### Scenario: Borrado y consulta posterior

- **GIVEN** entradas en memoria y persistidas
- **WHEN** se llama a `DELETE /api/history` y después a `GET /api/history`
- **THEN** `total` es 0 e `items` está vacío

### Requirement: El recuento y la página SHALL resolverse en una misma instantánea

`total` e `items` MUST proceder de una única lectura consistente del histórico: ninguna escritura concurrente (`append_update_history`) puede commitear entre el cálculo del recuento y la lectura de la ventana. Ambos valores MUST describir el mismo conjunto de filas; una respuesta internamente incoherente (recuento que incluye una fila ausente en `items` o al revés) MUST NOT devolverse.

La consistencia MUST obtenerse dentro de SQLite (transacción de lectura con instantánea estable, `BEGIN DEFERRED`) y MUST NOT requerir tomar el lock en memoria `update_history`, ni serializar a los escritores más allá de la duración de la lectura.

El envelope MUST seguir siendo `{items, total, page, page_size}` y, cuando el `offset` solicitado cae más allá del final del conjunto filtrado, la respuesta MUST devolver `items` vacío con el `total` real del conjunto.

#### Scenario: Append concurrente entre recuento y ventana

- **GIVEN** un histórico con 20 entradas y `page_size` mayor o igual que el total
- **WHEN** un worker persiste una entrada nueva mientras se sirve `GET /api/history`
- **THEN** la respuesta es coherente: o bien 20 elementos con `total` 20, o bien 21 con `total` 21, nunca `total` distinto de `items.len()`

#### Scenario: Página más allá del final

- **GIVEN** 25 entradas de histórico persistidas
- **WHEN** se llama a `GET /api/history?page=99&page_size=20`
- **THEN** `items` está vacío y `total` es 25

#### Scenario: Sin retener el lock en memoria

- **GIVEN** la pool de base de datos saturada
- **WHEN** se sirve `GET /api/history`
- **THEN** el handler no retiene el lock de `update_history` mientras espera una conexión
