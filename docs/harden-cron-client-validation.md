# harden-cron-client-validation

Follow-up de #141 (`validate-cron-client-ui`). Issue: #150.

## Qué es

Endurece la validación **cliente** de la expresión cron del formulario de updates, alineándola con la
gramática del backend (crate `cron 0.15`). Antes era solo estructural (5 campos + charset), de modo que un
cron como `99 0 * * *` o `30-10 * * * *` pasaba el pre-chequeo y el usuario solo veía el error tras el viaje
al backend (400). Ahora se validan **rangos y sintaxis por campo** en el cliente (feedback inmediato).

Durante la auditoría se detectó que el borrador inicial asumía día-de-semana `0–7`, cuando la crate usa
**`1–7`** (1 = domingo, `0` inválido). Esto implicaba:
- **Falso positivo**: `0 0 * * 0` (el cron del propio preset **«Cada domingo»**) pasaba el cliente y el
  backend lo rechazaba → **el preset estaba roto**. Se corrige a `0 0 * * SUN`.
- **Falsos negativos**: `?` y nombres largos (`MONDAY`, `JANUARY`…) son válidos en el backend pero se
  bloqueaban. Se admiten (nombres de forma **permisiva**).

## Decisiones (operador)

- Validar **por rangos/sintaxis**, **sin dependencias** nuevas.
- Sigue **bloqueando** el guardado (error inline).
- **Paridad razonable** con la crate: se arregla el rango de día-semana, se admiten `?` y nombres, se acota
  el paso. No se busca paridad **exacta** (`L`, `W`, `#`, alias raros como `JAN2`).

## Ficheros implicados

- `frontend/src/components/ConfigPage.tsx`:
  - Validación por campo (rangos por posición): minuto `0–59`, hora `0–23`, día-del-mes `1–31`,
    mes `1–12`, día-de-semana **`1–7`**.
  - Elementos: listas `,`; `*`; `?` (solo día-del-mes y día-de-semana); valores; rangos `a-b` (no
    invertidos en numéricos); pasos `/n` con `1 <= n <= máximo` del campo.
  - Nombres (solo mes y día-de-semana): alfabéticos de longitud ≥ 3, **permisivos** (superset de la crate).
- `frontend/src/cronPresets.ts`: `CRON_PRESETS` extraído de `ConfigPage.tsx` (para no exportar una constante
  desde un archivo de componente, que rompería fast-refresh). «Cada domingo» `0 0 * * 0` → `0 0 * * SUN`.
- `frontend/src/components/ConfigPage.test.tsx`: tests de inválidos (fuera de rango, día-semana `0`, rango
  invertido, paso cero/fuera de rango) y de válidos (`?`, nombres cortos y largos, listas, rangos); test de
  presets; test de backend-400 adaptado a un cron client-válido.

## Cómo funciona

Al editar/guardar el cron, `cronErrorFor` valida el valor normalizado (5 campos canónicos, con/sin
segundos) contra las reglas por campo. Si es inválido, se muestra el error inline y **no** se dispara el
`PUT /api/update-check/config`. Si el cliente lo acepta, el backend sigue siendo la **autoridad final**: si
responde `400 {error}`, la UI muestra ese mensaje.

## Variables / config

- Sin cambios de configuración. Backend: `backend/src/timezone.rs` (`is_valid_cron`, crate `cron 0.15`),
  sin modificar.

## Cómo habilitar / desplegar

Sin acción: es solo validación de UI.

## Verificación

- Frontend: `npx vitest run`, `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` limpios.
- Auditoría: `@react-reviewer` (hallazgos H1/H2/H3 corregidos; vectores verificados contra la crate).

## Caveats / follow-ups

- **Nombres permisivos**: cualquier token alfabético ≥ 3 letras en mes/día-semana pasa el cliente; un
  nombre que el backend no reconozca lo rechazará el backend (400) y su mensaje se muestra en la UI.
- No hay **paridad exacta** con la crate: `L`, `W`, `#`, combinaciones día/mes y alias no estándar
  (`JAN2`) quedan fuera; el backend sigue siendo la autoridad.
