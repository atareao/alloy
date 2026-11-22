# Spec Delta

## Purpose

Definir el comportamiento de la vista del histórico de actualizaciones: paginación en servidor con selector 10/20/50, elipsis real con tooltip en las columnas de texto y una única columna de versión.

## ADDED Requirements

### Requirement: La vista del histórico SHALL paginarse contra el servidor

`HistoryPage` MUST solicitar `GET /api/history` con `page` y `page_size` y MUST pintar únicamente los `items` recibidos, usando el `total` de la respuesta para el control de paginación. La paginación MUST resolverse en el servidor: la página MUST NOT recortar en cliente una lista completa.

#### Scenario: Carga inicial

- **GIVEN** que el usuario abre la vista del histórico
- **WHEN** se monta la página
- **THEN** se solicita `/api/history?page=1&page_size=20`
- **THEN** la tabla muestra los `items` de la respuesta y el `total` del control coincide con el de la respuesta

#### Scenario: Cambio de página

- **GIVEN** 25 entradas y la primera página visible
- **WHEN** el usuario pulsa la página 2
- **THEN** se solicita `/api/history?page=2&page_size=20`
- **THEN** la tabla muestra 5 filas

### Requirement: El selector de tamaño SHALL ofrecer 10, 20 y 50

El control de paginación MUST ofrecer los tamaños 10, 20 y 50 y MUST mostrar el tamaño efectivo y el total. Al cambiar el tamaño MUST resetear la página a 1 y MUST volver a consultar el servidor. El control MUST estar disponible tanto en la vista de escritorio como en la de móvil.

#### Scenario: Cambio de tamaño

- **GIVEN** el usuario está en la página 3 con tamaño 20
- **WHEN** selecciona el tamaño 50
- **THEN** se solicita `/api/history?page=1&page_size=50`

#### Scenario: Paginación en móvil

- **GIVEN** una pantalla de menos de 768 px
- **WHEN** se muestra la vista del histórico
- **THEN** el control de paginación está presente con los mismos tamaños

### Requirement: La página SHALL ser dueña de su estado y de su consulta

`HistoryPage` MUST mantener su propia página, tamaño y filtros, y MUST ser quien solicita el histórico al servidor. El componente contenedor MUST NOT mantener el array del histórico; MUST limitarse a señalar cuándo debe refrescarse, por ejemplo al terminar un batch.

#### Scenario: Refresco tras un batch

- **GIVEN** la vista del histórico abierta en la página 2
- **WHEN** termina un batch de actualizaciones y la señal de refresco cambia
- **THEN** se vuelve a solicitar la página 2 al servidor

#### Scenario: Borrado del histórico

- **GIVEN** el usuario pulsa borrar el histórico
- **WHEN** el servidor confirma el borrado
- **THEN** se solicita `/api/history?page=1&page_size=<tamaño actual>` y la tabla queda vacía

### Requirement: Las columnas de texto SHALL truncarse con elipsis real y tooltip

Las columnas `Container` e `Imagen` MUST truncar su contenido con elipsis real (CSS/Ant Design) cuando no quepa y MUST mostrar el valor completo en un tooltip. MUST NOT cortar el texto manualmente con `slice` ni añadir `"..."` a mano. Las celdas truncables MUST tener una anchura acotada para que la elipsis se aplique de verdad y MUST NOT provocar desbordamiento horizontal de la fila. En un contenedor flex la celda MUST además poder encogerse (`minWidth: 0`); en una tabla de anchura natural (`width: max-content`), donde la celda se dimensiona a su max-content, la cota de anchura es lo que hace efectiva la elipsis.

#### Scenario: Nombre largo

- **GIVEN** una entrada cuyo contenedor tiene un nombre que no cabe en la columna
- **WHEN** se renderiza la fila
- **THEN** el nombre se muestra con elipsis real, sin `slice` manual
- **THEN** el tooltip muestra el nombre completo

#### Scenario: Imagen larga

- **GIVEN** una entrada cuya imagen no cabe en la columna
- **WHEN** se renderiza la fila
- **THEN** la imagen se muestra con elipsis real y el tooltip contiene el valor completo

#### Scenario: Sin desbordamiento

- **GIVEN** un contenedor con un nombre muy largo
- **WHEN** se renderiza la fila
- **THEN** el ancho del contenido no excede el ancho de la celda

#### Scenario: Anchura acotada

- **GIVEN** una entrada cuyo contenedor tiene un nombre que no cabe en la columna
- **WHEN** se renderiza la fila
- **THEN** la celda truncable tiene una cota de anchura explícita
- **THEN** el contenido se muestra truncado con elipsis y no desborda la celda

### Requirement: La versión SHALL mostrarse en una única columna

La tabla MUST mostrar una sola columna de versión titulada `Versión`, en lugar de dos columnas separadas para la versión anterior y la nueva. La columna MUST mostrar la versión nueva en la línea superior y la versión anterior en la inferior, ambas con elipsis real y tooltip del valor completo. Un digest vacío MUST representarse como `-`. La tarjeta de la vista móvil MUST mostrar la misma composición.

#### Scenario: Entrada con ambas versiones

- **GIVEN** una entrada con digest anterior y nuevo
- **WHEN** se renderiza la fila
- **THEN** existe una única columna `Versión`
- **THEN** el digest nuevo aparece encima del anterior
- **THEN** ambos muestran el valor completo en su tooltip

#### Scenario: Digest vacío

- **GIVEN** una entrada con `old_digest` vacío
- **WHEN** se renderiza la fila
- **THEN** la línea correspondiente a la versión anterior muestra `-`

#### Scenario: Tarjeta móvil

- **GIVEN** una pantalla de menos de 768 px
- **WHEN** se muestra la tarjeta de una entrada
- **THEN** la versión nueva aparece encima de la anterior, con elipsis y tooltip

### Requirement: Las filas SHALL tener una clave estable

La tabla MUST asignar a cada fila una clave derivada de los datos de la entrada y MUST NOT usar la posición de la fila en la página, para que cambiar de página no reutilice filas de la página anterior.

#### Scenario: Cambio de página sin reutilizar filas

- **GIVEN** dos páginas con entradas distintas
- **WHEN** el usuario cambia de la página 1 a la 2
- **THEN** las filas renderizadas corresponden a las entradas de la página 2

### Requirement: La vista MUST NOT depender del stream en tiempo real

El histórico MUST seguir cargándose por REST y MUST NOT añadirse al `snapshot` del stream ni al long-poll. La paginación y los filtros del histórico MUST NOT alterar la consulta del stream ni la de la lista de contenedores.

#### Scenario: Sin cambios en el stream

- **GIVEN** el usuario navega a la vista del histórico
- **WHEN** cambia de página en el histórico
- **THEN** la conexión del stream no se reconecta por ese motivo

### Requirement: La vista SHALL permitir filtrar por texto y por estado

`HistoryPage` MUST ofrecer un campo de búsqueda de texto libre y un control para seleccionar uno o varios estados. El texto MUST aplicarse como filtro `q` y los estados seleccionados como `status` repetido, y la consulta al servidor MUST incluir ambos junto a `page` y `page_size`. La búsqueda MUST aplicar un retardo de 300 ms antes de consultar el servidor. Cambiar cualquiera de los dos filtros MUST resetear la página a 1. El filtrado MUST resolverse en el servidor: la página MUST NOT descartar en cliente ninguno de los `items` recibidos. Un filtro sin valor MUST NOT añadirse a la consulta.

#### Scenario: Búsqueda con debounce

- **GIVEN** la vista del histórico en la página 3
- **WHEN** el usuario escribe "web" en el buscador y deja de escribir durante 300 ms
- **THEN** se solicita `/api/history?page=1&page_size=20&q=web`

#### Scenario: Filtro por estado

- **GIVEN** el usuario selecciona los estados `error` y `rolled-back`
- **WHEN** se lanza la consulta
- **THEN** se solicita `/api/history?page=1&page_size=20&status=error&status=rolled-back`

#### Scenario: Filtros combinados y reset de página

- **GIVEN** la vista en la página 2 con el buscador vacío
- **WHEN** el usuario escribe "web" y selecciona el estado `error`
- **THEN** se solicita `page=1` con `q=web` y `status=error`

#### Scenario: Sin filtros

- **GIVEN** el buscador vacío y ningún estado seleccionado
- **WHEN** se consulta el histórico
- **THEN** la consulta sólo incluye `page` y `page_size`

#### Scenario: Filtrado en servidor

- **GIVEN** un filtro activo y una respuesta con `items`
- **WHEN** se renderiza la tabla
- **THEN** se pintan todos los `items` recibidos, sin descartar ninguno en cliente
