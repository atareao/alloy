# Proposal

## Why

Follow-up de #141 (`validate-cron-client-ui`). La validación cliente del cron es **solo estructural**
(5 campos + charset `^[0-9A-Za-z*,\-/]+$`): un cron como `99 99 99 99 99` o `30-10 * * * *` pasa el
pre-chequeo y el usuario solo descubre el error tras el viaje al backend (400). Endurecerla evita ese
viaje y da feedback inmediato.

## What Changes

- **Frontend** (`ConfigPage.tsx`): endurecer `isValidCronClient`/`cronErrorFor` para validar **rangos y
  sintaxis** por campo, **sin dependencias**:
  - 5 campos; cada uno admite `*`, valores/rangos/listas y pasos: `a`, `a-b`, `a/n`, `a-b/n`, `*/n`, listas
    con `,`.
  - Rangos: minuto `0–59`, hora `0–23`, día-del-mes `1–31`, mes `1–12` (o nombres `JAN..DEC`),
    día-de-semana `0–7` (o nombres `SUN..SAT`). Pasos `> 0`; rangos no invertidos (`a <= b`).
  - Mensaje de error inline (se mantiene el prefijo "expresión cron inválida").
- **Tests** (`ConfigPage.test.tsx`): fuera de rango, rango invertido y paso cero ⇒ error inline + PUT no
  disparado; crons válidos (`*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *`, `0 6 * * 1-5`) ⇒ sin error + PUT.
  Se adapta el test de "backend responde 400" para usar un cron **client-valid** (el mock simula el 400),
  preservando la cobertura de que la UI muestra el mensaje del backend.

## Capabilities

### Modified Capabilities

- `updates/update-check`: la validación cliente del cron pasa de estructural a **rangos + sintaxis**
  (sigue bloqueando el guardado). El backend no cambia.

### New Capabilities

- Ninguna.

## Impact

- **Frontend**: `ConfigPage.tsx` (+ tests). **Solo frontend**.
- **Backend**: sin cambios (`is_valid_cron` sigue siendo la autoridad final).
- **API**: sin cambios.
- **Fuera de alcance**: paridad exacta con la crate `cron` (p. ej. `?`, `L`, `W` no soportados; tampoco la
  validación de combinaciones día/mes).
