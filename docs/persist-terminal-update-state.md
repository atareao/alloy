# persist-terminal-update-state

Follow-up de la feature #125 (`update-rollback-guard`). Issue: #130.

## Qué es

Hace que los **guards de la guarda de arranque** de las auto-actualizaciones sobrevivan a un
reinicio del proceso de alloy, reconstruyéndolos desde el historial persistido en SQLite en lugar de
arrancar vacíos.

- `TerminalFailed` — contenedores en estado terminal `failed-stopped` (supresión D9).
- `RolledBackGuard` — contenedores con `rolled-back` y su digest intentado (anti-churn D10).

## Problema que resuelve

Antes, en `main.rs` ambos guards se creaban con `.default()` (vacíos) y nunca se reconstruían, así que
tras un reinicio se perdían la supresión terminal y la memoria de rollback, aunque las filas
`failed-stopped` / `rolled-back` seguían en `update_history`.

## Ficheros implicados

- `backend/src/updates/health_guard.rs` — constantes `STATUS_UPDATED` / `STATUS_ROLLED_BACK` /
  `STATUS_FAILED_STOPPED`; predicado `is_present`; `restore_guards_from_history(entries, present_names)`.
- `backend/src/db.rs` — `load_latest_history_per_container` (último estado por contenedor,
  independiente del `LIMIT 100` de `load_update_history`) + índice idempotente
  `idx_update_history_container_ts`.
- `backend/src/main.rs` — bootstrap: carga historial + nombres presentes en Docker (`all: true`) y
  reconstruye los guards; errores loggeados con `tracing::warn!` (fail-open).
- `backend/src/workers/scheduler.rs` — usa las constantes centralizadas y reutiliza `is_present`.

## Cómo funciona

1. **Al arrancar**: se lee, por contenedor, su último registro de `update_history`
   (`load_latest_history_per_container`, ordenado por `timestamp DESC, id DESC LIMIT 1`).
2. **Restauración con poda por presencia**: `restore_guards_from_history` aplica "último gana" y solo
   conserva contenedores **presentes** en el daemon (un `failed-stopped` está parado pero existe, por eso
   se lista con `all: true`). No depende de la tabla `containers`.
3. **Reconciliación en el primer ciclo**: la limpieza de runtime del scheduler (`!decision.has_update`,
   evaluada **cada ciclo** con el estado en vivo) elimina cualquier supresión restaurada que ya no
   corresponda a un update pendiente. El estado converge al real en el primer ciclo.

## Configuración

No añade ajustes. No hay nuevas columnas ni cambios de contrato API/frontend.

## Decisiones de diseño

- **Persistir en el historial, no en una tabla nueva** ni en `containers`: esta última **no se rellena en
  producción** (`save_containers` es código muerto; los `UPDATE containers` existentes son no-ops), por lo
  que no puede usarse como fuente de elegibilidad.
- **Gate por `has_update` descartado** por lo anterior; la convergencia se delega en la limpieza de
  runtime, que ya existía y está testeada.
- **Constantes centralizadas** para los estados terminales (un único origen).

## Verificación

- Backend: `cd backend && cargo test` → 409 OK; `just check` / `just lint` limpios.
- Auditoría `@rust-reviewer`: **APROBADO** (tras 2 rondas de cambios requeridos).

## Caveats / follow-ups

- **D9 por nombre**: si el runtime limpia `failed-stopped` sin escribir fila y tras reiniciar el registro
  vuelve a ofrecer update, la supresión se mantiene hasta **intervención manual** (comportamiento definido
  por D9). `rolled_back`, al ir keyed por digest, reintenta con un digest nuevo.
- **Reconciliación dependiente del registro**: con el registry caído, una supresión obsoleta se mantiene
  (inocuo: no hay update que proponer) y se reconcilia cuando vuelve la resolución remota.
- Índice `update_history(container, timestamp DESC, id DESC)` añadido para evitar O(N²) en arranque con
  historiales grandes.
