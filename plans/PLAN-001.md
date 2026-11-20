# PLAN-001 — Persistir estados terminales de update (rolled-back / failed-stopped)

- **Issue**: #130 (https://github.com/atareao/alloy/issues/130)
- **Change OpenSpec**: `persist-terminal-update-state`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/persist-terminal-update-state` desde `develop`; PR a `develop`.

## Objetivo

Que los guards de la guarda de arranque (`TerminalFailed` y `RolledBackGuard`) sobrevivan a un reinicio
del proceso de alloy reconstruyéndolos desde el historial persistido (`update_history`, SQLite), en lugar
de arrancar vacíos.

## Problema

- `backend/src/state.rs:21-22,32-33` — `TerminalFailed(HashSet<String>)` y `RolledBackGuard(HashMap<String,String>)`.
- `backend/src/main.rs:190-191` — se inicializan vacíos (`.default()`) y nunca se reconstruyen.
- Las filas `rolled-back` / `failed-stopped` SÍ están en `update_history` (SQLite `data/alloy.db`), pero
  no se usan para reconstruir el estado en memoria.
- Diseño archivado que lo dejó como follow-up: `openspec/changes/archive/2026-11-20-update-rollback-guard/design.md:137-138`.

## Alcance

1. Reconstruir guards al arrancar:
   - `TerminalFailed` ← contenedores cuyo ÚLTIMO registro de historial tiene `status = "failed-stopped"`.
   - `RolledBackGuard` ← contenedores cuyo último registro tiene `status = "rolled-back"`, con digest = `new_digest`.
2. Semántica "último registro gana": una fila terminal antigua anulada por una fila posterior (`updated`) NO suprime.
3. Replay independiente del truncado `LIMIT 100` de `load_update_history` (consulta del último estado por contenedor).
4. Poda coherente con runtime: conservar solo contenedores presentes en el daemon
   (`retain_present_terminal` / `retain_present_rolled_back`).
5. Centralizar constantes de estado terminal `"rolled-back"` / `"failed-stopped"`
   (hoy literales en `scheduler.rs:368,395,525,552` y `health_guard.rs:137-138`).

## Fuera de alcance

- Cambios de contrato API o de frontend. Nuevas columnas de esquema (se reutilizan `status` y `new_digest`).

## Ficheros implicados

- `backend/src/main.rs` (bootstrap: reconstruir guards tras cargar el historial).
- `backend/src/updates/health_guard.rs` (constantes + helper puro de replay).
- `backend/src/db.rs` / `backend/src/updates/history.rs` (consulta del último estado por contenedor).
- `backend/src/workers/scheduler.rs`, `backend/src/stacks.rs` (usar constantes).
- `backend/src/state.rs` (sin cambios de tipo salvo lo imprescindible).

## Plan TDD

### RED
- Tests de replay: `failed-stopped` → `TerminalFailed`; `rolled-back` → `RolledBackGuard[name]=new_digest`.
- Test "último gana"; historial vacío → guards vacíos; poda de contenedor ausente; resistencia al truncado.

### GREEN
- Constantes centralizadas + `restore_guards_from_history(entries, present_names)`.
- Consulta "último estado por contenedor" en la capa de persistencia.
- Wiring en `main.rs` para poblar `AppState`.

### REFACTOR / verificación
- `cd backend && just check` y `cargo test` en verde.
- Confirmar que no cambia el contrato API/frontend.

## Verificación de cierre

- [ ] Spec aprobado por el usuario (`.approved` presente).
- [ ] Tests RED → GREEN → REFACTOR completados.
- [ ] `just check` y `cargo test` en verde.
- [ ] `docs/persist-terminal-update-state.md` creado.
- [ ] `openspec archive persist-terminal-update-state` + limpieza del directorio del change.
- [ ] PR a `develop` mergeado e issue #130 cerrado.

## Referencias

- Feature previa: #125 / PR #129.
- Spec delta: `openspec/changes/persist-terminal-update-state/specs/updates/update-health-guard/spec.md`.
