# Tasks

## 1. Backend — RED

- [x] 1.1 Tests de deserialización de la query del histórico: defaults (`page=1`, `page_size=20`), escalares repetidos con last-wins, parámetros desconocidos ignorados y `status` repetible acumulado.
- [x] 1.2 Tests de validación: `page=0`, `page_size=30`, `page_size=abc` y `page=abc` responden `400`.
- [x] 1.3 Tests de paginación: página por defecto (25 entradas → 20 items y total 25), segunda página (5 items) y página fuera de rango (items vacío con total intacto).
- [x] 1.4 Tests de orden determinista: `timestamp` descendente y desempate por inserción descendente ante timestamps iguales.
- [x] 1.5 Tests de filtros: `q` case-insensitive sobre `container`, `image`, `old_digest` y `new_digest`, `status` repetible con OR, filtros combinados y `total` posterior al filtrado.
- [x] 1.6 Tests de `db`: `count_update_history` y `load_update_history_page` sobre SQLite real con más de 100 filas (verifica que ya no hay cap), con y sin filtros, y `LIMIT`/`OFFSET` correctos.
- [x] 1.7 Test de regresión: `delete_history_h` sigue vaciando memoria y persistencia, y el `GET` posterior devuelve `total` 0.
- [x] 1.8 Test de concurrencia: la lectura no retiene el lock del histórico en memoria con la pool saturada.
- [x] 1.9 Test de serialización del envelope `{items,total,page,page_size}`.

## 2. Backend — GREEN

- [x] 2.1 `models.rs`: struct de query del histórico con `Deserialize` manual (mismas tolerancias que `ContainerListQuery`), `normalize()` con defaults y `400`, y struct del envelope.
- [x] 2.2 `db.rs`: `count_update_history` y `load_update_history_page` con filtros, `LIMIT`/`OFFSET` y orden `timestamp DESC, id DESC`.
- [x] 2.3 `db.rs`: índice `idx_update_history_ts_id` como migración aditiva.
- [x] 2.4 `updates/history.rs`: `get_history_h` con `Query`, lectura contra la pool y envelope; eliminar el cap de 100 al servir.
- [x] 2.5 Mantener `delete_history_h` sin cambios de comportamiento y sin retener locks al esperar la pool.

## 3. Frontend — RED

- [x] 3.1 Tests de que `HistoryPage` solicita `/api/history?page=1&page_size=20` al montar y pinta los `items` con el `total` de la respuesta.
- [x] 3.2 Test del `Pagination`: el cambio de página consulta `page=2` y el cambio de tamaño a 50 resetea a `page=1&page_size=50`.
- [x] 3.3 Test de presencia del `Pagination` en la vista móvil.
- [x] 3.4 Test de elipsis: `Container` e `Imagen` usan elipsis real sin `slice` manual y el tooltip contiene el valor completo; test de no-desbordamiento de la celda con un nombre largo.
- [x] 3.5 Test de la columna única `Versión`: existe una sola columna, el digest nuevo va encima del anterior, ambos con tooltip y un digest vacío se muestra como `-`.
- [x] 3.6 Test de la composición de versión en la tarjeta móvil.
- [x] 3.7 Test de clave de fila estable: al cambiar de página se renderizan las entradas de la nueva página.
- [x] 3.8 Test del refresco: al cambiar la señal de refresco se reconsulta la página actual, y tras el borrado se consulta la página 1.
- [x] 3.9 Actualizar `App.test.tsx`: `App` ya no solicita `/api/history` y el mock del endpoint se ajusta al envelope.
- [x] 3.10 Tests de la UI de filtros: el buscador con debounce emite `q` y resetea a página 1; el selector de estado emite `status` repetido; sin filtros la consulta sólo lleva `page` y `page_size`; la tabla pinta todos los `items` recibidos sin filtrar en cliente.

## 4. Frontend — GREEN

- [x] 4.1 `historyQuery.ts`: construcción de la query (`page`, `page_size`, `q` y `status` repetido) con constantes compartidas de tamaños.
- [x] 4.2 `components/EllipsisText.tsx`: elipsis real con tooltip, `minWidth: 0` y variantes mono y secundaria.
- [x] 4.3 `HistoryPage.tsx`: estado de página, tamaño y filtros; `fetch` propio y `Pagination` con 10/20/50 en escritorio y móvil.
- [x] 4.4 `HistoryPage.tsx`: elipsis y tooltip en `Container` e `Imagen`.
- [x] 4.5 `HistoryPage.tsx`: columna única `Versión` (nueva encima de la antigua) en la tabla y en la tarjeta móvil; eliminar `shortDigest` y las dos columnas antiguas.
- [x] 4.6 `HistoryPage.tsx`: clave de fila compuesta y estable.
- [x] 4.7 `App.tsx`: eliminar el estado del histórico y los dos `fetch`; pasar `refreshToken` y ajustar `HistoryPageProps` y `types.ts`.
- [x] 4.8 `HistoryPage.tsx`: buscador con debounce de 300 ms y selector múltiple de estado, ambos con reset a página 1 y sin emitir parámetros vacíos.
- [x] 4.9 Cota de anchura explícita en las columnas `Container` e `Imagen` (la tabla es `width: max-content`, así que sin cota la elipsis no se dispara) y refuerzo de los tests de elipsis y de clave de fila para que fallen si la cota o la clave se degradan.

## 5. Verificación y cierre

- [x] 5.1 Backend: `cd backend && just check` limpio y `cargo test` en verde, sin bajar de los 541 tests actuales.
- [x] 5.2 Frontend: `pnpm exec tsc -b`, `pnpm run lint`, `pnpm run test` y `pnpm run build` en verde, sin bajar de los 209 tests actuales.
- [x] 5.3 Auditorías de `@rust-reviewer` y `@react-reviewer` sin hallazgos bloqueantes.
- [x] 5.4 Documentación: `docs/update-history-pagination.md` y actualización de `HANDOFF.md`.
- [ ] 5.5 Archivar el change y cerrar el issue #197 con el PR.

## 6. Fuera de alcance (anotado, no se ejecuta aquí)

- Poda o retención de la tabla `update_history`, que hoy crece sin límite.
- Migrar `ContainerRow` e `InspectModal` al componente `EllipsisText` compartido.
- Exponer el `id` de la entrada en la API.
