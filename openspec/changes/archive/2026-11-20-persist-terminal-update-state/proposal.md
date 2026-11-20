# Proposal

## Why

La guarda de arranque introducida en la feature #125 mantiene su estado **solo en memoria**:

- `backend/src/state.rs:21-22,32-33` — `TerminalFailed(HashSet<String>)` y `RolledBackGuard(HashMap<String,String>)`.
- `backend/src/main.rs:190-191` — al construir `AppState` se inicializan **vacíos** (`.default()`) y **nunca** se reconstruyen desde el historial.

Consecuencia: tras **reiniciar el proceso de alloy** (deploy, crash, reinicio del host) se pierde la
supresión anti-churn y el digest de rollback por contenedor. Un contenedor que quedó en `failed-stopped`
vuelve a reintentarse, y uno en `rolled-back` pierde la memoria del digest que ya falló —justo cuando
el historial persistido (`update_history` en SQLite) **sí conserva** esas filas. El propio diseño
archivado lo dejó como follow-up explícito (`archive/2026-11-20-update-rollback-guard/design.md:137-138`).

## What Changes

- **Reconstrucción de guards al arrancar** desde `update_history`:
  - `TerminalFailed` ← contenedores cuyo **último** registro de historial tiene `status = "failed-stopped"`.
  - `RolledBackGuard` ← contenedores cuyo último registro tiene `status = "rolled-back"`; el digest intentado se toma de `new_digest`.
- **Poda coherente con el runtime**: al restaurar, solo se conservan contenedores **presentes** en el
  daemon (equivalente a `retain_present_terminal` / `retain_present_rolled_back`).
- **Semántica de "último registro gana"**: un contenedor con una fila terminal antigua pero un registro
  posterior (p. ej. `updated`) NO queda suprimido.
- **Independencia del truncado**: el replay NO puede depender del `LIMIT 100` de `load_update_history`
  (debe leer lo necesario para derivar el último estado por contenedor de forma fiable).
- **Constantes centralizadas** para los strings `"rolled-back"` / `"failed-stopped"` (hoy repetidos en
  `scheduler.rs:368,395,525,552` y `health_guard.rs:137-138`).

## Capabilities

### Modified Capabilities

- `updates/update-health-guard`: la guarda pasa de estado puramente en memoria a **restaurable desde el
  historial persistido** al arrancar; se define la semántica de reconstrucción y de poda, y se centralizan
  las constantes de estado terminal.

### New Capabilities

- Ninguna.

## Impact

- **Backend**: `main.rs` (bootstrap: reconstruir guards tras cargar `update_history`), `updates/health_guard.rs`
  (constantes + helpers de replay), `db.rs`/`updates/history.rs` (consulta del último estado por contenedor
  si el truncado de 100 es insuficiente), `state.rs` (sin cambios de tipo salvo lo necesario).
- **API**: sin cambios de contrato (no hay nuevos endpoints ni campos visibles).
- **Frontend**: sin cambios (el historial ya muestra los estados `rolled-back` / `failed-stopped`).
- **Datos**: sin nuevas columnas previstas; se reutiliza `new_digest` y `status`.
- **Dependencias**: ninguna nueva (rusqlite/deadpool ya presentes).
