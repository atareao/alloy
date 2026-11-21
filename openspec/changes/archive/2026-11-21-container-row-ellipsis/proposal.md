# Proposal

## Why

Parte 1 del issue #166. En el listado de contenedores, el nombre y el estado no se truncan bien: en escritorio llevan la prop `ellipsis` de Ant Design pero la celda no fija ancho, así que el truncado puede no activarse; en móvil el truncado es manual (`c.name.slice(0, 9) + "..."`), que no es elipsis real ni respeta el ancho de la tarjeta y además corta el texto sin avisar del valor completo.

## What Changes

- Sustituir el truncado manual (`slice`) de nombre y estado por elipsis real de Ant Design en la fila, tanto en la variante móvil como en la de escritorio.
- Mostrar el valor completo en un `Tooltip` al truncar.
- Fijar el ancho/`minWidth` de los contenedores flex de nombre y estado para que el truncado sea efectivo, y que el resto de la fila (badge de actualización, spinner, enlace de Traefik, botón de expansión) no se comprima.
- Aplicar elipsis real al texto de progreso (`progress.checking`) en lugar del `slice(0, 20)`.

Fuera de alcance: la **imagen** del contenedor. La fila no la muestra hoy (vive en `InspectModal`); mostrarla requeriría un change incremental. Se documenta.

## Capabilities

### New Capabilities

- `frontend/container-list`: presentación del listado de contenedores.

### Modified Capabilities

Ninguna.

## Impact

- `frontend/src/components/ContainerRow.tsx` (y, si hace falta, el contenedor del grupo en `frontend/src/components/DashboardPage.tsx`).
- Tests: `frontend/src/components/ContainerRow.test.tsx`.
- Sin cambios de API, dependencias ni backend.
- Issue: #166 (parte 1 de 2).
