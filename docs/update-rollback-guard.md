# update-rollback-guard

## Qué es

Guarda de arranque para las auto-actualizaciones de contenedores. Sustituye la detección previa
(3 sondeos puntuales ~21 s + rollback por tag `rollback-<ts>` frágil, con fallo silencioso) por:

1. **Detección fiable de arranque sano**: `running` estable durante una ventana, con `RestartCount`
   sin subir y, si la imagen define `HEALTHCHECK`, estado `healthy`.
2. **Reintentos** de arranque hasta N (default 3), configurable global y por política.
3. **Rollback por digest exacto** de la imagen anterior (persistido antes de actualizar; no se
   elimina la imagen antigua hasta confirmar que la nueva está sana).
4. **Estado terminal** si el rollback tampoco arranca: contenedor **parado** (`failed-stopped`) con
   notificación de error; si el rollback sano, estado `rolled-back` con notificación warn.
5. **Sin churn**: ni `failed-stopped` ni `rolled-back` provocan reintentos en cada ciclo del worker
   (supresión por nombre/digest hasta intervención manual o nueva versión).

## Ficheros implicados

Backend (`backend/src/`):
- `updates/health_guard.rs` — lógica pura de la guarda: niveles de arranque, reintentos, rollback,
  supresión (`is_rolled_back_suppressed`, `record_rolled_back`, `next_step`, `outcome_after_rollback`,
  `decide_after_verify`, `guard_notify_level`).
- `containers.rs` — `inspect_runtime_health` (state/restart_count/health), `verify_stable_running`,
  `stop_container`.
- `updates/handlers.rs` — `apply_with_guard` (helper compartido worker + flujos manuales),
  `run_start_guard`, `rollback_to_digest`, `record_guard_outcome`.
- `workers/scheduler.rs` — cableado en el worker, supresión por ciclo, historial con `attempts`.
- `models.rs` (`Settings`/`UpdatePolicy`/`UpdateCheckConfig`/`UpdateHistoryEntry`),
  `db.rs` (migraciones idempotentes), `notifications.rs` (`notify_all_level`, `valet_level`),
  `admin.rs` (API de config y políticas), `state.rs` (`TerminalFailed`, `RolledBackGuard`), `main.rs`.
- `stacks.rs` — `update_stack_h` pasa por la guarda.

Frontend (`frontend/src/`):
- `types.ts`, `HistoryPage.tsx` (mapa `STATUS_LEVEL`, columna `Intentos`), `components/ConfigPage.tsx`,
  `components/PolicyActionButton.tsx`, `components/ContainerActions.tsx`.

## Cómo funciona

Al aplicar una política que recrea el contenedor (`PullRestart`/`PullRestartStack`) **con
`rollback_on_failure = true`**, `apply_with_guard` recrea → `verify_stable_running` (muestreo dentro
de la ventana) → reintentos hasta el máximo → rollback por digest → verificación post-rollback. Los
flujos manuales (`update_container_h`, `update_all_h`, `update_stack_h`) usan el mismo helper, por lo
que la imagen antigua solo se elimina tras confirmar la nueva sana. Un error de inspección de Docker
es **inconcluso** (se loguea `ERROR` y se omite el teardown; no provoca rollback/parada espurios).

Un `rolled-back` o `failed-stopped` se registra en el historial (`status`, `old_digest`, `new_digest`,
`attempts`) y se notifica (warn / error). La supresión (`terminal_failed`, `rolled_back`) evita
reintentos automáticos en ciclos posteriores hasta intervención del operador o cambio de versión.

## Configuración

Global (Settings / `GET|PUT /api/update-check/config`):
- `update_max_start_attempts` (default **3**)
- `update_start_attempt_interval_secs` (default **10**)
- `update_health_window_secs` (default **30**)

Por política (`UpdatePolicy`): overrides opcionales `max_start_attempts` y `health_window_secs`
(tienen prioridad sobre los globales). Editables desde ConfigPage (política por defecto) y por
contenedor (PolicyActionButton).

## Cómo habilitar / desplegar

La verificación ampliada y el rollback solo actúan cuando la política del contenedor tiene
`rollback_on_failure = true`; con `false` se conserva el comportamiento previo. No hay variables de
entorno nuevas. Migraciones de esquema idempotentes al arranque (`update_history.attempts`,
`update_policies.max_start_attempts`, `update_policies.health_window_secs`).

## Decisiones (ver `openspec/changes/update-rollback-guard/design.md`)

D1 un único rollback por digest; D2 lectura de `RestartCount`/`HealthStatus`; D3 ventana estable;
D4 reintentos; D5 niveles de notificación; D6 historial con `attempts`; D7 settings/política;
D8 UI; D9 supresión tras `failed-stopped`; D10 anti-churn tras `rolled-back`; D11 flujos manuales
heredan la guarda; D12 error de inspección ≠ no sano.

## Follow-ups

- Persistir `terminal_failed`/`rolled_back` en disco (hoy en memoria; se pierden al reiniciar).
- Limpiar la supresión también en `update_all_h`.
- `unwrap()` preexistentes en `handlers.rs` (pool/lock) fuera del alcance de este change.
- Reconciliar defaults de cron (`scheduler.rs` vs `admin.rs`) y `pull_timeout_secs` (600 vs 1800).
