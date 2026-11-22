# Paginación y filtros del histórico de actualizaciones

- **Issue:** #197
- **Change OpenSpec:** `openspec/changes/update-history-pagination/` (proposal, design con las decisiones D1–D7, specs de backend y frontend, tasks)
- **Specs archivados de referencia:** `openspec/specs/backend/container-pagination/spec.md`, `openspec/specs/frontend/container-list/spec.md`

## Qué es

El histórico de actualizaciones (`GET /api/history`) pasa de devolver una lista
recortada a 100 entradas a estar **paginado y filtrado en el servidor**, y la
vista (`HistoryPage`) pasa a ser la dueña de su estado de página y de su
consulta. Incluye la mejora de presentación de la tabla: elipsis real con
tooltip, una única columna `Versión` y una clave de fila estable.

**Problema que resuelve:** la tabla `update_history` crece sin límite (no hay
poda ni retención), pero al servir se recortaba a 100 entradas y al arrancar
también, así que había histórico inalcanzable desde la UI. Además, «Versión
anterior» y «Nueva versión» ocupaban dos columnas y el texto largo desbordaba la
fila.

## Contrato de la API

`GET /api/history` acepta:

| Parámetro | Valores | Por defecto | Notas |
|---|---|---|---|
| `page` | entero ≥ 1 | `1` | `page=0` o no numérico → `400` |
| `page_size` | `10`, `20`, `50` | `20` | cualquier otro valor o no numérico → `400` |
| `q` | texto libre | — | substring case-insensitive sobre `container`, `image`, `old_digest` y `new_digest` |
| `status` | repetible | — | coincidencia exacta, semántica **OR** entre los valores |

Tolerancias (idénticas a las ya fijadas para la lista de contenedores):
escalares repetidos → **gana el último**; parámetros desconocidos → ignorados;
`status` es el único repetible y **acumula** valores.

Respuesta (envelope):

```json
{ "items": [...], "total": 25, "page": 1, "page_size": 20 }
```

`total` es el recuento **después de aplicar los filtros**. `items` nunca expone
el `id` de la fila. Orden: `timestamp DESC, id DESC` (determinista, con
desempate por inserción).

## Ficheros implicados

### Backend

- `backend/src/models.rs` — `UpdateHistoryQuery` (con `impl Deserialize`
  **manual** por `visit_map`, para aceptar escalares repetidos con last-wins y
  `status` acumulado — un `derive` lo rechazaría), `UpdateHistoryParams`,
  `PagedUpdateHistory`, `UpdateHistoryQuery::normalize()` (defaults y `400`),
  `AppError::from_db`, y `page_offset` que pasa a `pub(crate)`.
- `backend/src/db.rs` — `count_update_history` y `load_update_history_page`,
  más `update_history_filter` (constructor **único** de la cláusula `WHERE` y
  sus valores, compartido por recuento y página para que `total` e `items` no
  puedan desincronizarse) y `escape_like` (escapa `\`, `%` y `_`). Índice
  aditivo `idx_update_history_ts_id ON update_history(timestamp DESC, id DESC)`
  con `CREATE INDEX IF NOT EXISTS`.
- `backend/src/updates/history.rs` — `get_history_h` devuelve el envelope; ya
  **no** recorta a 100 y **no** toma el lock del histórico en memoria.

### Frontend

- `frontend/src/HistoryPage.tsx` — dueña del estado (`page`, `pageSize`, `q`,
  `status`, `items`, `total`) y del `fetch`; `Pagination` de Ant Design con
  10/20/50 en escritorio y móvil; buscador con debounce de 300 ms y chips de
  filtro de estado (operables por teclado: `role="button"`, `tabIndex`,
  `aria-pressed`, Enter/Space); columna única `Versión`; mensaje de vacío
  diferenciado según haya filtros activos.
- `frontend/src/historyQuery.ts` (**nuevo**) — `buildHistoryQuery` (emite
  siempre `page` y `page_size`, y `q`/`status` sólo con valor) y `historyRowKey`
  (clave de fila derivada de los datos).
- `frontend/src/components/EllipsisText.tsx` (**nuevo**) — elipsis real con
  `Typography.Text ellipsis={{ tooltip }}`.
- `frontend/src/App.tsx` — deja de mantener el array del histórico y de hacer
  sus dos `fetch`; sólo pasa `refreshToken`, que incrementa al terminar un batch.
- `frontend/src/types.ts` — añade el tipo del envelope (`PagedHistory`).

## Cómo funciona

1. `HistoryPage` construye la query con `buildHistoryQuery` y la consulta con
   `apiFetch`.
2. El servidor cuenta con `COUNT(*)` y carga la página con `LIMIT`/`OFFSET`,
   ambos con el mismo `WHERE`.
3. El resultado se pinta tal cual: **la página no recorta ni filtra nada en
   cliente**.
4. Cambiar de tamaño de página o cambiar un filtro resetea a la página 1.
5. Cuando `App` termina un batch incrementa `refreshToken`, y `HistoryPage`
   reconsulta la página actual. Tras borrar el histórico, vuelve a la página 1.

## Variables/config

Ninguna nueva: no hay variables de entorno, flags ni configuración. El endpoint
usa el mismo `DbPool` y las mismas rutas de `updates/mod.rs` que antes.

## Cómo habilitar y desplegar

No requiere pasos especiales. El índice `idx_update_history_ts_id` se crea de
forma aditiva e idempotente al arrancar, junto al resto de migraciones del
esquema (no rompe bases de datos existentes). Nota: `CREATE INDEX` sobre una
tabla ya grande añade un coste único de arranque.

Verificación antes de desplegar:

```bash
cd backend && just check && cargo test
cd frontend && pnpm exec tsc -b && pnpm run lint && pnpm run test && pnpm run build
```

## Decisiones de diseño

- **La página se sirve desde SQLite**, no desde el vector en memoria: elimina el
  cap de 100 y, de paso, la lectura deja de tomar el lock del histórico en
  memoria (regla del issue #170: no retener el lock mientras se espera la pool
  de conexiones).
- **`q` es case-insensitive sólo en ASCII**: el `LIKE` de SQLite no aplica
  plegado Unicode (a diferencia del filtrado de la lista de contenedores, que
  usa `to_lowercase()`). Se acepta porque en este dominio es equivalente: los
  nombres de contenedor y las referencias de imagen están restringidos a ASCII
  por Docker y por la especificación de imágenes OCI, y los digests son
  hexadecimales. Filtrar en aplicación rompería la paginación en SQL.
- **La clave de fila se deriva de los datos**
  (`timestamp|container|image|status`) porque la API no expone el `id`; usar la
  posición en la página haría que React reutilizara filas al cambiar de página.
- **La elipsis en la tabla del histórico necesita una cota de anchura explícita
  por columna** (`maxWidth: 260`). La tabla se renderiza con `width: max-content`
  y `table-layout: auto`, así que cada celda se dimensiona a su max-content: sin
  cota el contenido nunca excede la celda y la elipsis no se dispara (verificado
  en Chromium real: un valor de 1687 px no truncaba y desbordaba el contenedor).
  `minWidth: 0` sólo actúa dentro de un contenedor flex (tarjeta móvil y columna
  `Versión`). Alternativas descartadas: `scroll={{ x: "max-content" }}` (inyecta
  la fila oculta `ant-table-measure-row` y duplica el texto de las cabeceras) y
  `table-layout: fixed` con reparto porcentual.

## Follow-ups conocidos

- `load_update_history` sigue truncando a **100 filas** al arrancar, porque solo puebla el vector en memoria que usan `export_config_h` y los `push` de los workers/scheduler. La vista ya no depende de él: pagina contra SQLite. Si en el futuro se quiere exportar el histórico completo, habrá que revisar ese punto.
- `AppError::Internal` (y por tanto `from_db`/`from_pool`) incluye el detalle del
  error en el cuerpo del `500`. Es un patrón **preexistente** de todo el
  proyecto; conviene corregirlo de forma global, no aquí.
- ~~`total` e `items` se obtienen en **dos consultas** (TOCTOU clásico de paginación:
  un `append` concurrente podía desincronizarlos y hacer que el cliente viera una
  página vacía o perdiera la fila recién insertada).~~ **Resuelto**: ambas
  consultas se ejecutan en una única transacción de lectura (`BEGIN DEFERRED`,
  WAL) en `db::load_update_history_page_with_total`, de modo que describen la
  misma instantánea sin bloquear a los escritores.
- La clave de fila puede colisionar en teoría si coinciden `timestamp`,
  `container`, `image` y `status` (timestamp con precisión de segundo). El `id`
  no se expone por decisión de diseño.
- La columna `Estado` no está acotada; el vocabulario de estados lo fija el
  backend, así que no es alcanzable con datos reales.
- La tarjeta móvil no tiene aserción de cota en jsdom (jsdom no calcula layout).
- `overlayStyle` está deprecado en antd v6 (a favor de `styles.root`); es
  preexistente.
- No hay poda ni retención de la tabla `update_history`.
- `ContainerRow` e `InspectModal` no se han migrado al nuevo `EllipsisText`
  (fuera de alcance de este cambio).

## Verificación

- Backend: `cargo test` → 576 passed / 0 failed; `just check` (fmt + clippy
  `-D warnings`) limpio.
- Frontend: `pnpm run test` → 233 passed / 0 failed (19 ficheros); `tsc -b`,
  `lint` y `build` limpios. Suite verificada estable en 5 ejecuciones
  consecutivas bajo carga de CPU.
