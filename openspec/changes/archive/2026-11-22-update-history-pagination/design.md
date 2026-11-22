# Design

## Context

- `GET /api/history` sirve hoy desde el vector en memoria `Arc<Mutex<Vec<UpdateHistoryEntry>>>` del `AppState`, ordenado por `timestamp` descendente y truncado a 100.
- Ese vector se siembra al arrancar con `load_update_history` (también con `LIMIT 100`) y crece con cada `append_update_history`, que **sí** persiste en SQLite sin cap.
- La tabla `update_history` no tiene poda ni retención: crece indefinidamente. Su índice actual es `idx_update_history_container_ts (container, timestamp DESC, id DESC)`, que no sirve para ordenar globalmente por `timestamp`.
- El histórico **no** se transporta por SSE: `StateResponse` sólo lleva `containers`, `total`, `page`, `page_size`, `summary` y `progress`. Llega al cliente por dos `fetch` REST en `App.tsx` (carga inicial y al terminar un batch).
- Precedente a replicar: `backend/container-pagination` y `frontend/container-list` (issue #166, parte 2): envelope `{items,total,page,page_size}`, `page_size` en {10,20,50}, tolerancias de query y `Pagination` de Ant Design con selector de tamaño.
- Regla de concurrencia vigente (issue #170): ningún handler puede retener `update_history.lock()` mientras espera una conexión de la pool.
- El export de configuración (`admin.rs`) sigue leyendo el vector en memoria; no se toca.

## Goals / Non-Goals

**Goals:**

- Paginar y filtrar el histórico **completo**, no un recorte de 100 filas.
- Mantener el mismo contrato, defaults y tolerancias que la lista de contenedores.
- Eliminar el corte manual de texto en la vista del histórico y unificar la columna de versión.
- No introducir acoplamiento con el stream en tiempo real.

**Non-Goals:**

- Poda o retención de `update_history` (sigue creciendo sin límite; se anota como follow-up).
- Eliminar el vector en memoria (lo siguen usando el export de configuración y los `push` de los workers).
- Migrar `ContainerRow` e `InspectModal` al componente de elipsis compartido (feature aparte).
- Accesibilidad por teclado de los tooltips de Ant Design más allá de lo que aporte el componente nuevo.

## Decisions

### D1 — La página se sirve desde SQLite, no desde el vector en memoria

El histórico es un log persistido, no un estado cacheado: el vector en memoria sólo contiene las últimas 100 filas previas al arranque más lo ocurrido en runtime, así que paginar sobre él daría un `total` falso y páginas incompletas. Se añaden `count_update_history` (con filtros) y `load_update_history_page` (con filtros y `LIMIT`/`OFFSET`) y el handler resuelve la página contra la pool.

Alternativa descartada: paginar en memoria sobre el vector, como hace la lista de contenedores. Es más barato, pero su `total` refleja el vector, no el histórico, y mantendría invisible todo lo anterior a las 100 filas del arranque.

Efecto colateral positivo: el handler deja de tomar `update_history.lock()` en la lectura, con lo que la regla del issue #170 se cumple por construcción.

### D2 — Filtros `q` y `status`

`q` es un substring case-insensitive sobre `container`, `image`, `old_digest` y `new_digest`. `status` es repetible con semántica OR y comparación exacta contra el valor almacenado; no se valida contra un vocabulario cerrado, igual que `state` en contenedores (los estados incluyen `success`, `error`, `rolled-back`, `failed-stopped` y la familia `update-check-*`). Los filtros se aplican antes de paginar y `total` es el recuento posterior.

Alternativa descartada: sólo paginación. Con ocho columnas y un histórico sin poda, la búsqueda es la mitad del valor de la vista; el patrón ya está resuelto en contenedores y reutilizarlo no añade conceptos nuevos.

La vista expone ambos filtros: un buscador con debounce de 300 ms y un selector múltiple de estado construido con el vocabulario ya presente en la página (`success`, `error`, `rolled-back`, `failed-stopped` y la familia `update-check-*`). El filtrado sigue siendo en servidor: la página sólo pinta los `items` recibidos.

La comparación de `q` es case-insensitive **en ASCII**: el `LIKE` de SQLite no aplica plegado Unicode (a diferencia de `to_lowercase()`, que sí usa el filtrado de la lista de contenedores). Se acepta y se documenta porque en este dominio es equivalente: los nombres de contenedor y las referencias de imagen están restringidos a ASCII por Docker y por la especificación de imágenes OCI, y los digests son hexadecimales. La alternativa —traer las filas y filtrar en aplicación con `to_lowercase()`— se descarta porque rompería la paginación en SQL.

### D3 — El estado de página vive en `HistoryPage`, no en `App`

En la lista de contenedores la página vive en `App` porque el `EventSource` necesita la misma query. El histórico no tiene stream, así que `HistoryPage` puede construir su query, hacer su `fetch` y guardar `page`, `page_size` y filtros. `App` deja de mantener el array del histórico y sólo pasa `refreshToken: number`, que se incrementa al terminar un batch para forzar la recarga.

Alternativa descartada: replicar el reparto de contenedores (`App` como dueño del estado). Añadiría props y estado en `App` para un dato que ya no se comparte con nadie.

### D4 — Componente compartido `EllipsisText` para elipsis y tooltip

El patrón actual del proyecto es inline (`Typography.Text ellipsis={{ tooltip }}` en `ContainerRow` e `InspectModal`), pero `HistoryPage` necesita el mismo comportamiento en dos columnas de la tabla y en la tarjeta móvil, así que se extrae `frontend/src/components/EllipsisText.tsx` (props: `value`, `mono?`, `secondary?`, `style?`) con elipsis real, `minWidth: 0` y tooltip. Se usa en `HistoryPage`; `ContainerRow` e `InspectModal` se dejan como están para acotar el radio de cambio.

La elipsis de las columnas `Container` e `Imagen` exige una cota de anchura explícita por columna. La tabla del histórico se renderiza con `width: max-content` y `table-layout: auto`, así que cada celda se dimensiona a su max-content: sin cota el contenido nunca excede la celda y la elipsis no se dispara (verificado en Chromium real: un valor de 1687 px no trunca y desborda el contenedor; con `max-width: 250px` trunca y no desborda). `minWidth: 0` sólo actúa dentro de un contenedor flex, por eso basta en la tarjeta móvil y en la columna `Versión` (que ya lleva `maxWidth: 260`) pero no en una celda de tabla. Alternativas descartadas: `scroll={{ x: "max-content" }}` (inyecta la fila oculta `ant-table-measure-row` y duplica el texto de las cabeceras) y `table-layout: fixed` con reparto porcentual (fuerza anchos fijos y empeora el resto de columnas).

### D5 — Orden determinista `timestamp DESC, id DESC` e índice nuevo

`timestamp` es texto ISO-8601, de modo que el orden lexicográfico equivale al cronológico (es el que ya usa el código). Como dos entradas pueden compartir segundo, se desempata por `id DESC`, que es la clave primaria. Para que `LIMIT/OFFSET` no degrade, se añade `idx_update_history_ts_id ON update_history(timestamp DESC, id DESC)` como migración aditiva (`CREATE INDEX IF NOT EXISTS`, el mismo mecanismo que el índice existente).

### D6 — Clave de fila estable

`UpdateHistoryEntry` no expone `id`, así que la tabla no puede usar la posición de la fila en la página como clave (React reutilizaría filas entre páginas). Se usa una clave compuesta determinista a partir de los campos de la entrada (`timestamp` + `container` + `image` + `status`), sin tocar el modelo ni el export de configuración.

Alternativa descartada: exponer `id` en la API. Obligaría a cambiar el modelo persistido y el payload del export de configuración para un dato que sólo sirve como clave de React.

### D7 — `Versión` unifica `old_digest` y `new_digest`

Una sola columna, sin `dataIndex`, con `render(_, record)`: la versión nueva arriba y la antigua debajo, ambas en monoespaciada, con elipsis real y tooltip del valor completo. Un digest vacío se representa como `-` (hoy `shortDigest` devuelve `-` para vacío y corta a 20 caracteres con `"..."`, y eso se elimina). La tarjeta móvil muestra la misma composición.

## Risks / Trade-offs

- **Coste por consulta**: `COUNT(*)` más `LIMIT/OFFSET` sobre una tabla sin poda crecen con el tiempo. Mitigación: el índice nuevo evita el orden en memoria; la poda se anota como follow-up.
- **Cambio incompatible** del endpoint: cualquier cliente que esperase un array se rompe. No hay consumidores conocidos fuera de `HistoryPage`.
- **Pérdida del `shortDigest`**: la tabla mostrará digests completos truncados por CSS, y el tooltip conserva el valor íntegro.
- **Reescritura de los 8 tests existentes** de `history.rs`, que pasan a necesitar pool y envelope; hay riesgo de perder cobertura de `delete_history_h` si no se migran con cuidado.
