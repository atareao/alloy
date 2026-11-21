# reconcile-update-defaults

Tercer y último follow-up de #125 (`update-rollback-guard`). Issue: #135.

## Qué es

Deja un **único origen** para los defaults de configuración de updates y alinea backend (API +
ejecución), docs y frontend, eliminando literales duplicados y en conflicto.

## Valores canónicos

| Ajuste | Valor | Const |
|---|---|---|
| `update_check_cron` | `0 6 * * *` (diario 6:00) | `DEFAULT_UPDATE_CHECK_CRON` |
| `pull_timeout_secs` | `600` (10 min) | `DEFAULT_PULL_TIMEOUT_SECS` |
| `check_interval_ms` | `2000` | `DEFAULT_CHECK_INTERVAL_MS` |

## Problema que resuelve

- **cron**: API/UI anunciaban `0 */6 * * *` pero el worker real caía a `0 0 * * *`.
- **pull_timeout_secs**: docs/etiqueta UI decían `1800`; la ejecución real usaba `600` en 7 sitios.
- **check_interval_ms**: `unwrap_or(2000)` duplicado.
- Sin const compartida (la guarda de arranque sí la tenía).

## Ficheros implicados

- `backend/src/models.rs` — consts + helpers `Settings::effective_update_check_cron()`,
  `effective_pull_timeout_secs()`, `effective_check_interval_ms()` (único punto de aplicación del fallback).
- `backend/src/admin.rs` — GET `/api/update-check/config` devuelve cron/pull/check **efectivos**.
- `backend/src/workers/scheduler.rs`, `updates/handlers.rs`, `stacks.rs` — usan los helpers.
- `frontend/src/components/ConfigPage.tsx` — cron inicial/placeholder `0 6 * * *`, timeout `600`,
  etiqueta `Default: 600 (10 min)`, fallback al vaciar `600`.

## Cómo funciona

Todos los puntos de uso delegan en los helpers `effective_*` de `Settings`, que aplican el fallback
canónico cuando el valor es `None` **o vacío** (`update_check_cron` con `""` cae al default, alineado con
la normalización vacío→`None` de `db.rs`). El **GET** expone los valores efectivos; el **PUT** devuelve lo
persistido (eco), sin ripple al frontend (el struct permanece en `Option`).

## Decisiones de diseño

- **Único origen en consts** en `models.rs`, replicando el patrón ya usado por la guarda de arranque
  (`DEFAULT_UPDATE_MAX_START_ATTEMPTS`, etc.).
- **Fallback vía helpers `effective_*`** en lugar de `unwrap_or(literal)` disperso.
- **cron canónico `0 6 * * *`** decidido por el operador (diario a las 6:00).
- El frontend mantiene valores literales alineados (el formulario precarga el efectivo vía GET; el
  literal solo cubre el transitorio previo a la carga).

## Verificación

- Backend: `cargo test` → **418 OK**; `just check` limpio. `rg` sin literales en producción.
- Frontend: `npx vitest run` → **102 OK**; `pnpm exec tsc -b` y `pnpm run lint` limpios.
- Auditorías: `@rust-reviewer` y `@react-reviewer` → **APROBADO**.

## Cómo habilitar / desplegar

Sin acción: los defaults aplican automáticamente cuando no hay valor guardado. Un valor explícito previo
en `settings` se respeta (no se migra).

## Caveats / follow-ups

- **Validación de cron en el PUT**: hoy un cron inválido se persiste y solo produce `warn!` en runtime
  (`admin.rs`). Follow-up.
- **Duplicación residual en el frontend**: los literales del formulario podrían eliminarse exponiendo los
  defaults por API. Follow-up.
- Flake de test frontend detectado durante el PR: issue #134 (ajeno a este cambio).
