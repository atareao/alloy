# Proposal

## Why

Follow-up de #141 (`validate-cron-client-ui`). La validación cliente del cron es **solo estructural**
(5 campos + charset): un cron como `99 99 99 99 99` o `30-10 * * * *` pasa el pre-chequeo y el usuario solo
descubre el error tras el viaje al backend (400). Endurecerla evita ese viaje y da feedback inmediato.

**Revisión tras auditoría** (`@react-reviewer`) verificada empíricamente contra la crate `cron 0.15.0`
(la misma del backend): el borrador inicial asumía día-de-semana `0–7` y no admitía `?`/nombres largos, lo
que producía **falsos positivos** (p. ej. `0 0 * * 0`, el cron del propio preset «Cada domingo», que el
backend rechaza) y **falsos negativos** (`?`, `MONDAY`… válidos en el backend pero bloqueados en cliente).
Se corrige el rango a **`1–7`** (1 = domingo, `0` inválido), se admiten `?` y nombres (permisivos), se acota
el paso y se arregla el preset roto.

## What Changes

- **Frontend** (`ConfigPage.tsx`): endurecer `isValidCronClient`/`cronErrorFor` para validar **rangos y
  sintaxis** por campo, **sin dependencias**, coherente con la crate `cron`:
  - 5 campos; cada uno admite `*`, `?` (solo día-del-mes y día-de-semana), valores, rangos `a-b`, listas
    `,` y pasos `/n` (`*/n`, `a/n`, `a-b/n`).
  - **Rangos**: minuto `0–59`, hora `0–23`, día-del-mes `1–31`, mes `1–12`, día-de-semana **`1–7`** (no
    `0–7`). Rangos no invertidos (`a <= b`).
  - **Pasos**: entero con `1 <= n <= máximo` del campo (minuto 59, hora 23, día-del-mes 31, mes 12,
    día-semana 7).
  - **Nombres** (solo mes y día-de-semana): alfabéticos de longitud ≥ 3, **permisivo** (superset de la
    crate) para no bloquear nombres válidos.
  - Mensaje de error inline (mantiene el prefijo "expresión cron inválida").
- **Frontend** (`ConfigPage.tsx`): arreglar el preset «Cada domingo» `0 0 * * 0` → `0 0 * * SUN` (el
  actual es rechazado por el backend).
- **Tests** (`ConfigPage.test.tsx`): fuera de rango (`99 0 * * *`), día-de-semana `0` (`0 0 * * 0`), rango
  invertido (`30-10 * * * *`), paso cero/fuera de rango (`*/0`, `*/60`) ⇒ error + PUT no disparado;
  válidos (`*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *`, `0 6 * * 1-5`, `0 6 ? * *`, `0 6 * * MON-FRI`,
  `0 6 1,15 * *`) ⇒ PUT; nombres largos (`0 6 * * MONDAY`, `0 6 1 JANUARY *`) ⇒ PUT; presets válidos. Se
  adapta el test de "backend responde 400" a un cron **client-válido** (mock 400).

## Capabilities

### Modified Capabilities

- `updates/update-check`: la validación cliente del cron pasa de estructural a **rangos + sintaxis**
  (sigue bloqueando el guardado) y se arregla el preset «Cada domingo».

### New Capabilities

- Ninguna.

## Impact

- **Frontend**: `ConfigPage.tsx` (+ tests). **Solo frontend**.
- **Backend**: sin cambios (`is_valid_cron` sigue siendo la autoridad final).
- **API**: sin cambios.
- **Fuera de alcance**: paridad **exacta** con la crate (p. ej. `L`, `W`, `#`, combinaciones día/mes, y
  alias de nombres no estándar como `JAN2`); los nombres son permisivos a propósito.
