# Tasks — persist-terminal-update-state

## 1. RED (tests que fallan)

- [ ] 1.1 Backend: test de replay — último registro `failed-stopped` ⇒ contenedor en `TerminalFailed`.
- [ ] 1.2 Backend: test de replay — último registro `rolled-back` ⇒ `RolledBackGuard[name] = new_digest`.
- [ ] 1.3 Backend: test "último gana" — fila terminal antigua + fila posterior `updated` ⇒ NO suprimido.
- [ ] 1.4 Backend: test historial vacío ⇒ guards vacíos (no regresión del comportamiento actual).
- [ ] 1.5 Backend: test de poda — contenedor suprimido ausente del daemon ⇒ descartado al restaurar.
- [ ] 1.6 Backend: test de truncado — estado terminal fuera de las 100 filas más recientes ⇒ se restaura igual.
- [ ] 1.7 Backend: test de constantes — los estados terminales se comparan vía constante centralizada.

## 2. GREEN (implementación mínima)

### 2.1 Constantes y helpers
- [ ] 2.1.1 `updates/health_guard.rs`: constantes `STATUS_ROLLED_BACK` / `STATUS_FAILED_STOPPED`
      (reemplazar literales en `guard_history_status` y en `scheduler.rs`).
- [ ] 2.1.2 `updates/health_guard.rs`: función pura de replay `restore_guards_from_history(
      entries, present_names) -> (HashSet<String>, HashMap<String,String>)`.

### 2.2 Persistencia / consulta
- [ ] 2.2.1 `db.rs`/`updates/history.rs`: consulta del **último estado por contenedor**
      (independiente de `LIMIT 100`).
- [ ] 2.2.2 `main.rs`: tras cargar historial, reconstruir `TerminalFailed`/`RolledBackGuard`
      y pasarlos al `AppState`.

### 2.3 Integración
- [ ] 2.3.1 Verificar que `scheduler.rs` y `stacks.rs` usan las constantes y que el replay no duplica lógica.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` (fmt + clippy en verde).
- [ ] 3.2 `cd backend && cargo test` (todos en verde; sumar los nuevos).
- [ ] 3.3 Sin cambios de contrato API/frontend; confirmar que no rompe endpoints.
- [ ] 3.4 Docs: actualizar/crear `docs/<feature>.md` al archivar.

## 4. Archive

- [ ] 4.1 `openspec archive persist-terminal-update-state` tras aprobación + implementación.
