# Elipsis en nombre/estado del contenedor

Change: `container-row-ellipsis` · Issue #166 (parte 1) · Plan `plans/PLAN-022.md`

## Qué es

Parte 1 del issue #166. Sustituye el truncado manual (`slice` + `"..."`) del nombre, el estado y el texto de progreso en la fila de contenedor por **elipsis real de Ant Design** con tooltip del valor completo, tanto en la variante móvil como en la de escritorio.

## Ficheros implicados

- `frontend/src/components/ContainerRow.tsx` — nombre, estado y progreso (variantes móvil y escritorio).
- `frontend/src/components/ContainerRow.test.tsx` — test de truncado/elipsis.
- `frontend/src/components/DashboardPage.tsx` — nombre del stack: `minWidth: 0, maxWidth: "100%"`.

## Cómo funciona

- **Nombre y estado:** `Typography.Text` con `ellipsis={{ tooltip: <valor> }}` y `minWidth: 0` (antes `minWidth: 60` impedía encoger). Se eliminaron `c.name.slice(0, 9) + "..."` (móvil), `c.status.slice(0, 17) + "..."` y el ternario `isMobile ? slice : full`.
- **Progreso:** se eliminó `progress.checking.slice(0, 20)`; el `Tag` de progreso usa `maxWidth: 160`, un `Spin` con `flexShrink: 0` y el texto con `ellipsis` + `minWidth: 0`, dentro del `Tooltip` ya existente.
- **Elementos no truncables con `flexShrink: 0`:** punto de estado, badge `!` de `has_update`, `Spin` de `updating`, enlace/botón de Traefik y botón de expansión.
- Se eliminó el `title={c.name}` / `title={c.status}` redundante que producía doble tooltip (antd + nativo del navegador).

## Variables/config

Ninguna.

## Cómo verificar

```bash
cd frontend
pnpm run test        # 179 tests
pnpm exec tsc -b
pnpm run lint
pnpm run build
```

## Decisiones

- La **imagen** del contenedor NO se renderiza en la fila (queda fuera de alcance; vive en `InspectModal`). Mostrarla requeriría un change incremental.
- En jsdom el Tooltip emergente de antd no es verificable (no hay layout → `isEllipsis` false → `disabled`); los tests comprueban el texto completo en el DOM y la clase `ant-typography-ellipsis`, y lo dejan documentado con un comentario.

## Follow-ups

- Parte 2 de #166: paginación en servidor 10/20/50, variante A: `page`/`page_size` en `GET /api/containers`, `/api/state` y `/api/stream` + filtros en servidor.
- Mostrar la imagen en la fila (change incremental).
- Warnings de `oxlint` (#179).
