# Imagen del contenedor en `InspectModal`

- **Issue:** #188 (follow-up de #166 parte 1)
- **Change OpenSpec:** `openspec/changes/inspect-modal-image/` (capability `ui/inspect-modal`)
- **Plan:** `plans/PLAN-026.md`

## Qué es

Endurece el render de la imagen del contenedor en la pestaña «General» de
`InspectModal`: la imagen principal se trunca con elipsis real de Ant Design y
expone el valor completo en un tooltip, y el campo tiene un fallback a
`image:tag` cuando el `inspect` de Docker no devuelve la imagen. El bloque
«Información adicional» y el enlace «Ver en registry» quedan estrictamente
condicionados a que existan sus datos.

Fuera de alcance: mostrar la imagen en la fila del listado
(`docs/container-row-ellipsis.md`), que ya tiene su propio truncado.

## Ficheros implicados

| Fichero | Rol |
|---|---|
| `frontend/src/components/InspectModal.tsx` | Implementación: helper `InfoRow`, elipsis + tooltip, fallback y bloques condicionales |
| `frontend/src/components/InspectModal.test.tsx` | Tests de los escenarios (elipsis/tooltip, fallback, bloque adicional, registry) |
| `frontend/src/components/testFixtures.ts` | Fixture `makeInspectData` reutilizado por los tests (sin cambios) |

## Cómo funciona

1. **Helper `InfoRow`** (componente local, mismo fichero): encapsula el patrón
   repetido `Flex` + etiqueta (`width: 140`, `flexShrink: 0`) + valor. El
   contenedor del valor es un `Flex` con `flex: 1`, `minWidth: 0` y
   `data-testid="info-row-value"` para que su contenido pueda encoger y para
   poder asertar el valor real en los tests. Cada fila gana ese `Flex` wrapper
   respecto a la versión anterior, pero etiqueta, valor y jerarquía accesible
   (`Text`, `Tag`, `Link`) se mantienen iguales.
2. **Imagen principal:** `Typography.Text` con
   `ellipsis={{ tooltip: imageValue }}` y `style={{ minWidth: 0 }}`. Ant Design
   añade la clase `ant-typography-ellipsis` y, cuando el texto desborda, muestra
   `…` y el valor completo en tooltip. El `minWidth: 0` del wrapper es lo que
   permite que la elipsis se active dentro del `Flex`.
3. **Fallback:** `imageValue = inspectData.image || (containerInfo ?
   `${containerInfo.image}:${containerInfo.image_tag}` : "")`.
   Sin `inspectData.image` y sin `containerInfo` el campo queda vacío sin romper
   el modal; con `containerInfo` se muestra `image:tag`.
4. **Bloque «Información adicional»:** se renderiza solo si `containerInfo` no
   es `null`, y dentro de él la fila «Registry:» con el enlace «Ver en registry»
   solo si `containerInfo.registry_url` es una cadena no vacía.

## Escenarios cubiertos por tests

En `frontend/src/components/InspectModal.test.tsx`:

- `trunca la imagen principal con elipsis y conserva el valor completo`:
  la imagen larga queda en un `.ant-typography` con la clase
  `ant-typography-ellipsis` y el texto completo sigue en el DOM.
- `ofrece el valor completo como tooltip cuando la imagen no cabe`: se simula
  el desbordamiento midiendo `getBoundingClientRect` (jsdom no calcula layout)
  y se comprueba que al pasar el ratón aparece `role="tooltip"` con el valor
  completo. El stub emula `isEleEllipsis` de `antd/es/typography/Base/util.js`.
- `hace encogible la imagen (minWidth: 0) para que la elipsis no desborde`:
  comprueba en el nodo `.ant-typography` de la imagen `minWidth: 0px` y en el
  wrapper del valor `flex: 1` + `minWidth: 0px` (mismo enfoque que
  `ContainerRow.test.tsx`, ya que jsdom no tiene layout).
- `usa el fallback image:tag cuando inspectData.image está vacío`:
  `image: ""` + `containerInfo` `redis`/`7-alpine` → se ve `redis:7-alpine`.
- `renderiza sin errores y deja el valor de la imagen vacío sin containerInfo`:
  localiza el `data-testid="info-row-value"` de la fila «Imagen:» y comprueba
  que su `textContent` es `""`.
- `no muestra el bloque ni la fila de registry sin containerInfo`.
- `no muestra el enlace al registry cuando registry_url está vacío`.
- `muestra el enlace al registry con registry_url no vacío` (y comprueba `href`).

También se mantienen los tests previos (carga, error, estado vacío, datos
generales, navegación entre pestañas y cierre).

## Decisiones

- **Elipsis nativa de AntD** en lugar de CSS manual, para reutilizar el patrón ya
  usado en `ContainerRow` (`ellipsis={{ tooltip }}` + `minWidth: 0`).
- **`flexShrink: 0` en la etiqueta** y `minWidth: 0` en el contenedor del valor:
  la etiqueta mantiene su ancho de 140 px y la elipsis recae siempre en el
  valor, no en la etiqueta.
- **Fallback a nivel de render** (no en el backend): el backend no se toca; si
  Docker no devuelve `image`, el frontend compone `image:tag` con los datos que
  ya recibe en `containerInfo`.
- **Tooltip verificado con métricas simuladas**: jsdom devuelve rects a cero, así
  que el test de tooltip stubea `getBoundingClientRect` solo durante ese test
  (`vi.restoreAllMocks()` en `afterEach`). El resto de tests se limitan a la
  clase de elipsis y al texto completo, que es lo observable de forma estable.
- **Sin `@testing-library/user-event`**: no está instalado en el proyecto, así
  que el hover del tooltip se dispara con `fireEvent.mouseEnter` (patrón ya
  usado en este fichero).

## Follow-ups

- **Tooltip no accesible por teclado:** `Typography.Text` con `ellipsis` no
  expone el texto truncado a lectores de pantalla ni por foco (falta
  `tabIndex={0}` y un `aria-label`). Es una limitación de AntD ya presente en
  `ContainerRow`; se resolvería de forma transversal en ambos componentes.
- **El campo `ID:` sigue sin elipsis:** un ID de imagen (`sha256:…`) puede
  desbordar la fila; conviene aplicarle el mismo `ellipsis` + `minWidth: 0`.
- Valorar añadir `@testing-library/user-event` como devDependency para
  interacciones más realistas (hover/click) en toda la suite.
- Si en el futuro el listado necesita mostrar la imagen, reutilizar el mismo
  patrón de `InfoRow`/elipsis en lugar de duplicar estilos.
- `Table rowKey` del modal usa el parámetro `index` (deprecado en AntD v6):
  warning preexistente, fuera del alcance de este change.