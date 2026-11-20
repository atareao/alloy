# Tasks — persist-terminal-update-state

## 1. RED (tests que fallan)

- [x] 1.1 Backend: test de replay — último registro `failed-stopped` ⇒ contenedor en `TerminalFailed`.
- [x] 1.2 Backend: test de replay — último registro `rolled-back` ⇒ `RolledBackGuard[name] = new_digest`.
- [x] 1.3 Backend: test "último gana" — fila terminal antigua + fila posterior `updated` ⇒ NO suprimido.
- [x] 1.4 Backend: test historial vacío ⇒ guards vacíos (no regresión del comportamiento actual).
- [x] 1.5 Backend: test de poda — contenedor suprimido ausente del daemon ⇒ descartado al restaurar.
- [x] 1.6 Backend: test de truncado — estado terminal fuera de las 100 filas más recientes ⇒ se restaura igual.
- [x] 1.7 Backend: test de constantes — los estados terminales se comparan vía constante centralizada.

### 1b. RED (ronda de auditoría — poda por presencia y reconciliación)

- [x] 1b.1 Revert a 2 args `restore_guards_from_history(entries, present_names)`: poda **solo por
      presencia**, sin gate `containers.has_update` (tabla no rellena en producción).
- [x] 1b.2 Scheduler: supresión restaurada + `has_update = false` en vivo ⇒ el primer ciclo la
      elimina (`test_terminal_failed_first_cycle_cleanup_removes_stale_suppression`).
- [x] 1b.3 Scheduler: supresión vigente + `has_update = true` en vivo ⇒ se mantiene (D9)
      (`test_terminal_failed_first_cycle_keeps_suppression_when_update_pending`).
- [x] 1b.4 "Último gana" de `rolled_back` anulado por un `updated` posterior ⇒ NO restaura.
- [x] 1b.5 `rolled-back` con `new_digest == ""` no rompe el replay.
- [x] 1b.6 Predicado compartido `is_present` (misma semántica que la poda del runtime).
- [x] 1b.7 BD: a igual `timestamp`, el desempate por `id DESC` gana la fila más reciente.

## 2. GREEN (implementación mínima)

### 2.1 Constantes y helpers
- [x] 2.1.1 `updates/health_guard.rs`: constantes `STATUS_ROLLED_BACK` / `STATUS_FAILED_STOPPED`
      (reemplazar literales en `guard_history_status` y en `scheduler.rs`).
- [x] 2.1.2 `updates/health_guard.rs`: función pura de replay `restore_guards_from_history(
      entries, present_names) -> (HashSet<String>, HashMap<String,String>)`.

### 2.2 Persistencia / consulta
- [x] 2.2.1 `db.rs`/`updates/history.rs`: consulta del **último estado por contenedor**
      (independiente de `LIMIT 100`).
- [x] 2.2.2 `main.rs`: tras cargar historial, reconstruir `TerminalFailed`/`RolledBackGuard`
      y pasarlos al `AppState`.
- [x] 2.2.3 `main.rs`: poda por presencia (lista Docker `all: true`); fail-open con `tracing::warn!`
      ante error del historial o del listado Docker. La reconciliación de supresiones obsoletas la
      hace el scheduler en su primer ciclo (`has_update` en vivo).
- [x] 2.2.4 `db.rs`: índice idempotente `idx_update_history_container_ts` en `apply_schema`.

### 2.3 Integración
- [x] 2.3.1 Verificar que `scheduler.rs` y `stacks.rs` usan las constantes y que el replay no duplica lógica.
- [x] 2.3.2 Predicado compartido `is_present` reutilizado por la poda del runtime (`retain_present_*`).
- [x] 2.3.3 `notifications.rs` usa las constantes `STATUS_*` (sin literales de estado terminal en producción).

## 3. REFACTOR / verificación

- [x] 3.1 `cd backend && just check` (fmt + clippy en verde).
- [x] 3.2 `cd backend && cargo test` (todos en verde; sumar los nuevos).
- [x] 3.3 Sin cambios de contrato API/frontend; confirmar que no rompe endpoints.
- [ ] 3.4 Docs: actualizar/crear `docs/<feature>.md` al archivar.

## 4. Archive

- [ ] 4.1 `openspec archive persist-terminal-update-state` tras aprobación + implementación.
