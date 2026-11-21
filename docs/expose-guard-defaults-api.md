# expose-guard-defaults-api

Follow-up de #139 (`expose-update-defaults-api`). Issue: #148.

## Qué es

Elimina el **hardcodeo** de los defaults de la **guarda de arranque** en el frontend: se exponen por API
desde el punto único del backend y la UI los consume, igual que ya se hizo con los 3 canónicos de la
revisión.

## Decisiones

- Se **amplía** (aditivo) el endpoint existente `GET /api/update-check/defaults` con 3 campos; **no** se
  toca el contrato del GET/PUT `/api/update-check/config`.
- Alcance: los 3 defaults de la guarda (`update_max_start_attempts`, `update_start_attempt_interval_secs`,
  `update_health_window_secs`).
- El fallback de los campos de la guarda al vaciarlos toma el valor de la API; se eliminan los literales
  `3/10/30`.

## Ficheros implicados

- `backend/src/models.rs` — `UpdateDefaults` ampliado con los 3 campos + `canonical()` poblándolos desde
  las consts (`DEFAULT_UPDATE_MAX_START_ATTEMPTS`, `DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS`,
  `DEFAULT_UPDATE_HEALTH_WINDOW_SECS`). Cero literales.
- `backend/src/admin.rs` — test del endpoint ampliado.
- `frontend/src/types.ts` — `interface UpdateDefaults` ampliada.
- `frontend/src/components/ConfigPage.tsx` — los 3 `InputNumber` de la guarda usan `defaults?.<campo> ??
  prev` (updater funcional) al vaciarse.

## Cómo funciona

`GET /api/update-check/defaults` devuelve ahora
`{ cron, pull_timeout_secs, check_interval_ms, update_max_start_attempts: 3,
update_start_attempt_interval_secs: 10, update_health_window_secs: 30 }`, construido desde las consts. El
formulario los carga al montar y los usa como fallback al vaciar los campos de la guarda (conservando el
valor previo si la API aún no cargó o falló). El endpoint es **aditivo**.

## Variables / config

- Fuente de verdad: consts `DEFAULT_*` en `backend/src/models.rs:272-283`.

## Cómo habilitar / desplegar

Sin acción: el endpoint queda disponible automáticamente (bajo el middleware de auth existente).

## Verificación

- Backend: `cargo test` → **432 OK**; `just check` limpio.
- Frontend: `npx vitest run` → **113 OK**; `pnpm exec tsc -b`, `pnpm run lint` y `pnpm run build` limpios.
- Auditorías: `@rust-reviewer` y `@react-reviewer`.

## Caveats / follow-ups

- `check_interval_ms` se expone pero la UI no lo edita (issue #149).
- La validación cliente del cron es estructural, no por rangos (issue #150).
