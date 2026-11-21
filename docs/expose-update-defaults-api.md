# expose-update-defaults-api

Follow-up de #135 (`reconcile-update-defaults`). Issue: #139.

## Qué es

Elimina el **hardcodeo** de los defaults de configuración de updates en el frontend: el backend los
expone por API desde su **punto único** (las consts de `models.rs`) y la UI los consume.

## Decisiones

- Endpoint **nuevo** `GET /api/update-check/defaults` (struct propio), sin tocar el contrato del
  GET/PUT `/api/update-check/config`.
- Alcance: **solo los 3 canónicos** (`cron`, `pull_timeout_secs`, `check_interval_ms`).
- El formulario **depende de la API** para placeholder/etiqueta/fallback; se eliminaron los literales.
- El requisito `Consistencia de los defaults entre backend y UI` del spec se **modificó** (ya no exige
  mostrar los canónicos antes de cargar la API).

## Ficheros implicados

- `backend/src/models.rs` — struct `UpdateDefaults { cron, pull_timeout_secs, check_interval_ms }`
  (serde) + `UpdateDefaults::canonical()`, poblado **desde las consts**
  (`DEFAULT_UPDATE_CHECK_CRON`, `DEFAULT_PULL_TIMEOUT_SECS`, `DEFAULT_CHECK_INTERVAL_MS`). Cero literales.
- `backend/src/admin.rs` — handler `get_update_defaults_h` + ruta `GET /api/update-check/defaults`.
- `frontend/src/types.ts` — `interface UpdateDefaults`.
- `frontend/src/components/ConfigPage.tsx` — carga los defaults y los usa en el **placeholder** del cron,
  la **etiqueta** del timeout y el **fallback** al vaciar; init neutro (`""`/`0`).

## Cómo funciona

`GET /api/update-check/defaults` devuelve `{ "cron": "0 6 * * *", "pull_timeout_secs": 600,
"check_interval_ms": 2000 }`, construido desde las consts. El formulario los carga al montar y los usa
para el placeholder/etiqueta/fallback en lugar de literales. El endpoint es **aditivo**; el GET/PUT de
config no cambia.

## Variables / config

- Fuente de verdad: consts `DEFAULT_*` en `backend/src/models.rs`.

## Cómo habilitar / desplegar

Sin acción: el endpoint queda disponible automáticamente (bajo el middleware de auth existente).

## Verificación

- Backend: `cargo test` → **432 OK**; `just check` limpio.
- Frontend: `npx vitest run` → **103 OK**; `pnpm exec tsc -b` y `pnpm run lint` limpios.
- Auditorías: `@rust-reviewer` y `@react-reviewer` → **APROBADO**.

## Caveats / follow-ups

- Defaults de la **guarda** (`3/10/30`): siguen hardcodeados en el frontend. Follow-up.
- `check_interval_ms`: se expone pero la UI no lo edita.
