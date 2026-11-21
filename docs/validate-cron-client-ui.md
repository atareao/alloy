# validate-cron-client-ui

Pendiente de la serie de cron/updates (#125 → #135 → #137). Issue: #141.

## Qué es

Validación **cliente** del cron en el formulario de configuración de updates: error **inline** y
**bloqueo del guardado** si es inválido, y **normalización** de expresiones de 6 campos a 5. Antes, el
usuario solo descubría el error tras un PUT rechazado con 400.

## Decisiones (operador)

- Normalizar **6→5 en el frontend**: al detectar 6 campos, eliminar el de **segundos** (el primero).
- Validación cliente **ligera** (regex/estructura), **sin dependencias nuevas**.
- **Bloquear el guardado + error inline** si no pasa la validación cliente.
- El **backend no cambia**: sigue aceptando solo 5 campos (como en #137).

## Ficheros implicados

- `frontend/src/components/ConfigPage.tsx`:
  - `normalizeCron(expr)`: trim; si 6 campos, quita el de segundos → 5 canónicos.
  - `isValidCronClient(expr)` + `cronErrorFor(expr)`: 5 campos + charset `^[0-9*,\-/]+$`.
  - Estado `cronError` + error inline (`status="error"` + `Typography.Text type="danger" role="alert"`).
  - `onChange` recalcula el error; `onBlur` normaliza; `saveUpdateCheck` normaliza y **bloquea** el PUT si inválido.

## Cómo funciona

Al editar el campo, se recalcula el error en vivo (estado `cronError`). Al salir (`onBlur`) se aplica la
normalización 6→5. Al guardar, se normaliza el valor y, si es inválido, se muestra el error y **no** se
envía el `PUT`; si es válido, se envía el cron normalizado. El backend sigue siendo la **autoridad
final** (un cron con estructura válida pero rango imposible lo rechaza con 400).

## Verificación

- Frontend: `npx vitest run` → **108 OK**; `pnpm exec tsc -b` y `pnpm run lint` limpios; `pnpm run build` OK.
- Auditoría: `@react-reviewer`.

## Caveats / follow-ups

- La validación de cliente es **estructural**: no valida rangos (minuto/hora/día) ni nombres de mes/día;
  el backend decide.
- Un cron con charset válido pero rango imposible (p. ej. `99 99 99 99 99`) pasa el cliente y se rechaza
  en el backend (400) — es el caso usado para cubrir el 400 tras añadir la validación cliente.
