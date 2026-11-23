# Docs — ui-pagination-and-vertical-tabs (issue #220)

## Qué es

Feature que ajusta la interfaz del dashboard y configuración:

1. **Dashboard sin paginación**: el listado de contenedores muestra toda la flota
   usando el nuevo modo `all=true` del backend (`/api/containers?all=true`).
2. **Historial con pageSize 10**: la página de eventos abre siempre con 10 registros
   (antes dependía de un valor no especificado; ahora `DEFAULT_HISTORY_PAGE_SIZE=10`).
3. **Pestañas verticales en Notificaciones**: Telegram, Matrix y Valet dispuestos
   en un panel lateral izquierdo (`<Tabs tabPosition="left">`).
4. **Pestañas verticales en Actualizaciones**: Revisión y Política en las mismas tabs verticales.

## Ficheros implicados

### Backend (Rust)

| Fichero | Qué cambió |
|---|---|
| `backend/src/models.rs` | Campo `all: bool` en `ContainerListQuery`, `ContainerListParams`. Nueva ruta en `normalize()` para `all=true` que salta validación de page/page_size. `paginate_containers()` devuelve la lista completa cuando `all=true` (sin paginar). Visitor manual actualizado. |
| `backend/src/containers.rs` | Handler `state_h` pasa `all` a `ContainerListParams`; endpoint `/api/containers` acepta `all=true` y responde sin envelope paginado. Tests `test_list_containers_all_*` cubren todos los escenarios del spec delta. |
| `backend/src/events.rs` | `stream_h` reenvía `all` al stream SSE; función `first_stream_snapshot` testeada también con `all=true`. |

### Frontend (React + TS + Ant Design v6)

| Fichero | Qué cambió |
|---|---|
| `frontend/src/containerQuery.ts` | Eliminada dependencia de `Pagination`; query siempre envía `{ all: true }`. |
| `frontend/src/historyQuery.ts` | Nueva constante `DEFAULT_HISTORY_PAGE_SIZE = 10`. Query usa este valor para la primera petición. |
| `frontend/src/App.tsx` | Estado global de `containerQuery` sin page/page_size; handlers passthrough simples para filtros; prop muerta `containersLoaded` eliminada. |
| `frontend/src/components/DashboardPage.tsx` | Eliminado estado de paginación; lista plana renderizada sin `Pagination`; key estable añadida a filas (`c.id`). |
| `frontend/src/components/ContainerTable.tsx` | Sin props de paginación (`total`, `page`, `pageSize`, `onPageChange`, `onPageSizeChange` eliminadas). Tags de filtro de estado con a11y completo (`role="button"`, `aria-pressed`, `tabIndex`, `onKeyDown`). Switch "Solo pendientes" con `<label>` asociado. |
| `frontend/src/components/NotificationsTab.tsx` | Convertido a `<Tabs tabPosition="left">` con items Telegram/Matrix/Valet. Test propio con a11y verificada. |
| `frontend/src/components/UpdatesTab.tsx` | Convertido a `<Tabs tabPosition="left">` con items Revisión/Política. Test propio con a11y verificada. |
| `frontend/src/HistoryPage.tsx` | PageSize inicial fijado a 10 con opciones 10/20/50. Filtros de estado con mismo patrón a11y que ContainerTable. |

### Tests

| Fichero | Contenido |
|---|---|
| `backend/tests/containers.rs` | 6 tests nuevos para `all=true`: retorna cada container, aplica filtros, ignora page invalid, mantiene paginación cuando all=false. |
| `backend/tests/events.rs` | Tests de snapshot SSE con `all=true` y sin `all` (paginado). |
| `frontend/src/App.test.tsx` | URL de containers cambiada a `/api/containers?all=true`. Ajuste de asserts de pagination a null. |
| `frontend/src/components/ContainerTable.test.tsx` | Eliminadas assertions de paginación. Nuevos tests a11y: tags clicables + teclado, label→switch. |
| `frontend/src/components/DashboardPage.test.tsx` | Sin props de paginación. Test anti-key-warnings de React añadido. |
| `frontend/src/components/ConfigPage.test.tsx` | Migrado a `@testing-library/react` (72/72 tests). |
| `frontend/src/components/NotificationsTab.test.tsx` | Nuevo: verifica tabs position left, defaults, cambios de pestaña, accesibilidad. |
| `frontend/src/components/UpdatesTab.test.tsx` | Nuevo: verifica tabs position left, defaults, cambios de pestaña, accesibilidad. |
| `frontend/src/HistoryPage.test.tsx` | Tests de pageSize inicial 10, total de filas, opciones de tamaño. |
| `frontend/src/useRealtime.test.tsx` | Verificación de reabertura con query modificada (all=true). |

## Cómo funciona

### Modo `all=true` en el backend

```
/api/containers?all=true       → 200 { containers: [...] }        (lista plana)
/api/containers?page=2         → 200 { containers: [...], page: 2, ... }  (paginado)
```

El campo `all` se deserializa manualmente en `ContainerListParams::deserialize` mediante un
visitor que delega a `bool` y captura valores inválidos como error de serialización (400).

En `normalize()`, si `all == true` se asignan `page=1` y `page_size` al largo de items
(devueltos como `page_size` real), y se salta la validación de `page == 0` y rangos de
`page_size`. Luego `paginate_containers()` decide la rama según `params.all`.

Los endpoints `/api/state` y `/api/stream` reenvían `all` intacto, por lo que el SSE
y state snapshot reciben la misma lógica.

### Dashboard sin paginación (frontend)

`containerQuery.ts` construye siempre `GET /api/containers?all=true`. No hay estado de
`page` ni `pageSize` en `App.tsx`. `DashboardPage.tsx` recibe los datos directamente en
el componente y los renderiza planos en una tabla. Filtrado cliente-side: búsqueda debounced
+ estado filter `stateFilter` + switch "solo pendientes", ambos derivan en un
`listKey` único para invalidar la caché react-query.

### Historial con pageSize 10

`historyQuery.ts` exporta `DEFAULT_HISTORY_PAGE_SIZE = 10`. Este valor se usa como
estado inicial de `pageSize` en `HistoryPage`. La query se reconstruye en cada cambio
de pageSize (invalida react-query automáticamente). Opciones del dropdown: `[10, 20, 50]`.

### Pestañas verticales

```tsx
<Tabs tabPosition="left" defaultActiveKey={key}>
  <Tabs.TabPane tab={<span role="img">…</span>} key={key} />
  …
</Tabs>
```

Ant Design v6 requiere `tabPosition="left"` explícito porque el comportamiento por defecto
es `"top"`. Los paneles inactivos usan `role="tabpanel" aria-hidden="true"` con clase
`ant-tabs-content-hidden` (no `.ant-tabs-tabpane` como en v5). Los tests acceden con
`getByRole("tabpanel", { name })` para el activo y `closest('[role="tabpanel"]')` para
el inactivo.

## Variables / configuración

No hay variables nuevas. Solo cambia la estructura del objeto `ContainerListQuery` del backend
(agregando `all`) y los hooks de consulta frontend.

## Decisiones

1. **`all` como `bool` vs enum `Pagination`**: Se eligió un campo `bool` simple dentro de
   `ContainerListParams` ya que el struct existe solo para la query y el código es corto.
   Un enum (`Pagination::All` vs `Pagination::Page`) habría sido más robusto pero hubiera
   requerido cambios estructurales mayores sin beneficio tangible inmediato. Queda como nota
   para una refactorización futura si el sistema crece.

2. **a11y consistente entre Historial y Dashboard**: El Historial recibió tratamiento a11y
   completo para sus filtros de estado (tags clicables con keyboard + label para switches).
   Se replicó exactamente ese patrón en el Dashboard para mantener consistencia visual y
   funcional.

3. **`containersLoaded` eliminada**: Con la paginación fuera, este flag ya no tenía utilidad.
   Se eliminó totalmente junto con las props de paginación restantes.

4. **antd v6 `tabPosition="left"` incondicional**: Aunque en móviles consume ancho valioso,
   el spec lo exige así. Si en el futuro se quiere responsive switching, habría que modificar
   el spec y la implementación.

## Follow-ups (deuda registrada)

| Prioridad | Elemento | Razón |
|---|---|---|
| Baja | `all` como `enum Pagination` en vez de `bool` | Invariante mantenida solo por convención, no por tipo. |
| Baja | Mensaje de "flota vacía" en Dashboard | Ahora carga todo de golpe; vacío no notificado. |
| Baja | Test router-level de `/api/stream` | Solo test e2e vía handler; path idéntico a `/api/state` sí testea. |
| Media | Responsive de tabs verticales en móvil | `tabPosition="left"` fijo; seguir UX cuando sea necesario. |
| Media | Long-poll fallback test con `all=true` | `useRealtime.ts` pasa query al polling, ningún test lo asierte. |
| Extra | Grupo de stack completo en tests | Tests ejercitan `noStack` pero no grupos con réplicas reales. |

## Cómo habilitar / desplegar

Sin pasos especiales. La feature está incluida en el build normal del frontend y las rutas
del backend aceptan automáticamente el nuevo parámetro `all=true`. Deploy estándar con
`docker compose up --build` o `pnpm run build && cargo build`.

## Cierre

Issue: #220 · Change SDD: `openspec/changes/ui-pagination-and-vertical-tabs/` · Rama:
`feature/ui-pagination-and-vertical-tabs` · Commits tras merge.
