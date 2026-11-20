# Proposal

## Why

Las auto-actualizaciones no detectan de forma fiable un contenedor que **no arranca**: hoy solo se hacen 3 sondeos en ~21 s (un crash-loop posterior pasa desapercibido), el rollback usa un tag `rollback-<ts>` frágil sin verificar que la imagen anterior arranque, y si el rollback falla solo se deja un `warn` en el log —sin estado terminal ni notificación—. Un contenedor puede quedar reiniciándose en bucle hasta pararlo a mano.

## What Changes

- **Detección de arranque correcto**: `state == running` **estable** durante una ventana (sin reinicios, `RestartCount` sin subir) y, si la imagen define `HEALTHCHECK`, además `healthy`.
- **Reintentos de arranque hasta N** (default **3**, configurable global + override por política) con espera entre intentos.
- **Rollback por digest exacto** persistido antes de actualizar; la imagen anterior **no** se elimina hasta confirmar que la nueva está sana.
- **Verificación post-rollback**: si la imagen anterior arranca sana → estado `rolled-back` (notificación **warn**); si **tampoco** arranca → contenedor **parado**, estado terminal `failed-stopped`, notificación **error**.
- **Historial** amplía estados y guarda `old_digest`/`new_digest`, nº de intentos y fase final.
- **Nuevos settings**: `update_max_start_attempts` (default 3), `update_start_attempt_interval_secs`, `update_health_window_secs`; y los overrides correspondientes en `UpdatePolicy`.
- **Notificaciones de update**: rollback (warn) y fallo terminal (error) vía `notify_all_state` (telegram/matrix/webhook/valet).

## Capabilities

### New Capabilities

- `updates/update-health-guard`: verificación de arranque sano, reintentos N, rollback por digest exacto, parada terminal y notificación ante fallo.

### Modified Capabilities

- `updates/update-check`: la aplicación de políticas deja de reportar fallos solo en logs; integra el health-guard y expone resultados (rollback/fallo) en el historial y en las notificaciones.

## Impact

- **Backend**: `workers/scheduler.rs`, `updates/handlers.rs`, `updates/common.rs`, `containers.rs`, `models.rs`, `notifications.rs`, y persistencia de `old_digest`/estado en `db`.
- **API**: `/api/history` (nuevos estados y campos), `/api/update-check/config` (nuevos campos), `UpdatePolicy` (overrides de reintentos/ventana).
- **Frontend**: `ConfigPage.tsx`, `HistoryPage.tsx`, `types.ts`, `ContainerActions`/`PolicyActionButton` (campos y estados nuevos).
- **Dependencias**: ninguna nueva prevista (Bollard ya disponible).
