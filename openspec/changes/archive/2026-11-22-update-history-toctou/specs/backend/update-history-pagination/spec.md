# Spec Delta: backend/update-history-pagination

## ADDED Requirements

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