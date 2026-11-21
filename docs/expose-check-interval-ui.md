# expose-check-interval-ui

Follow-up de #135/#139/#148. Issue: #149.

## Qué es

Permite editar desde el formulario el **intervalo de comprobación** (`check_interval_ms`): el intervalo
en milisegundos entre verificaciones de digest de cada contenedor. El backend ya aceptaba y devolvía el
campo; faltaba la UI. Se edita en **segundos** y se convierte a milisegundos al guardar.

## Decisiones (operador)

- **Editar en la UI** (no dejarlo interno).
- Unidad: **segundos** enteros (conversión `*1000` al enviar, `/1000` al cargar).
- Etiqueta desde `GET /api/update-check/defaults` (en segundos), sin literales.

## Ficheros implicados

- `frontend/src/types.ts` — `check_interval_ms?: number | null` en `UpdateCheckConfig`.
- `frontend/src/components/ConfigPage.tsx`:
  - Estado `checkIntervalSecs` (segundos; init neutral `0`).
  - Carga desde el GET de config: `Math.round(check_interval_ms / 1000)`.
  - `InputNumber` (`min={1}`, `precision={0}`) con etiqueta desde `defaults.check_interval_ms / 1000`.
  - Envío: `check_interval_ms: checkIntervalSecs * 1000` en el `PUT`.

## Cómo funciona

El control vive en el bloque de revisión (solo monta con el auto-update habilitado). Al abrir el formulario
muestra el valor actual convertido a segundos; al guardar se envía en milisegundos. La etiqueta refleja el
default de la API (`2000 ms` → `2 s`).

## Variables / config

- Backend: `check_interval_ms` (`DEFAULT_CHECK_INTERVAL_MS = 2000`), `effective_check_interval_ms()`.

## Cómo habilitar / desplegar

Sin acción: el campo ya existe en el GET/PUT y en `/api/update-check/defaults`.

## Verificación

- Frontend: `npx vitest run` → **116 OK**; `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` limpios.
- Auditoría: `@react-reviewer`.

## Caveats / follow-ups

- Edición en **segundos enteros**: un valor almacenado no múltiplo de `1000` ms se redondea al mostrar/guardar.
- **Asimetría GET/PUT**: el `GET` devuelve el valor **efectivo** (`effective_check_interval_ms()` ⇒ `2000`
  si no hay valor guardado), mientras que el `PUT` devuelve el valor **crudo** (`Option`, `null` si nunca se
  guardó). La UI **siempre** envía el campo (⇒ el `PUT` devuelve un `Some`), y al re-sincronizar solo aplica
  el valor si no es `null`, conservando el mostrado. No requiere acción.
- Validación cliente del cron por rangos: issue #150.
