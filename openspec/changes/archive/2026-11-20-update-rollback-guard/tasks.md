# Tasks — update-rollback-guard

## 1. RED (tests que fallan)

- [x] 1.1 Backend: helper de salud — test de `inspect_runtime_health` que expone `state`,
      `restart_count` y `health: Option<String>` (None ⇒ sin HEALTHCHECK).
- [x] 1.2 Backend: guarda — test de que un `running` con `RestartCount` que sube dentro de la
      ventana NO se considera sano (crash-loop).
- [x] 1.3 Backend: guarda — test de que con HEALTHCHECK solo se considera sano si termina `healthy`.
- [x] 1.4 Backend: reintentos — test de que un arranque sano en el intento 2-3 no dispara rollback.
- [x] 1.5 Backend: reintentos — test de que al agotar N se inicia rollback por digest exacto.
- [x] 1.6 Backend: rollback — test de que la referencia usada es el digest exacto persistido
      (no un tag `rollback-<ts>`).
- [x] 1.7 Backend: post-rollback — test de rollback sano ⇒ `rolled-back` + notificación `warn`.
- [x] 1.8 Backend: post-rollback — test de rollback fallido ⇒ `failed-stopped` + contenedor parado
      + notificación `error`.
- [x] 1.9 Backend: override por política — test de que `max_start_attempts` de `UpdatePolicy` gana
      sobre el default global.
- [x] 1.10 Backend: persistencia — test de entrada de historial con `attempts` y `old_digest`/`new_digest`.
- [x] 1.11 Backend: notificación — test de `valet_level`/mapeo: `rolled-back → warning`,
      `failed-stopped → critical`.
- [x] 1.12 Backend: API — test de roundtrip `GET/PUT /api/update-check/config` con los 3 campos nuevos.
- [x] 1.13 Frontend: test de mapeo de los estados nuevos en `HistoryPage` (`rolled-back`,
      `failed-stopped`) y columna de intentos.
- [x] 1.14 Frontend: test de `ConfigPage` que renderiza/guarda los nuevos campos.

## 2. GREEN (implementación mínima)

### 2.1 Modelos y persistencia
- [x] 2.1.1 `models.rs`: `Settings` + `update_max_start_attempts` (default 3),
      `update_start_attempt_interval_secs`, `update_health_window_secs` (`serde(default)`).
- [x] 2.1.2 `models.rs`: `UpdatePolicy`/`UpdatePolicyReq` + `max_start_attempts: Option<u32>`,
      `health_window_secs: Option<u64>`.
- [x] 2.1.3 `models.rs`: `UpdateCheckConfig` + los 3 campos globales.
- [x] 2.1.4 `models.rs`: `UpdateHistoryEntry` + `attempts: u32` (`serde(default)`).
- [x] 2.1.5 `db.rs`: migraciones idempotentes (`update_history.attempts`,
      `update_policies.max_start_attempts`, `update_policies.health_window_secs`).
- [x] 2.1.6 `db.rs`: `load_settings`/`save_settings` y `load_update_policies`/`save_update_policy`
      con los campos nuevos.

### 2.2 Inspección de salud (Bollard)
- [x] 2.2.1 `containers.rs`: nuevo `inspect_runtime_health(docker, name) -> Result<R, AppError>`
      con `state`, `restart_count`, `health: Option<String>`.
- [x] 2.2.2 `containers.rs`: helper de parada (`stop_container`) reutilizable para `failed-stopped`.

### 2.3 Guarda (nuevo módulo, p. ej. `updates/health_guard.rs`)
- [x] 2.3.1 `verify_stable_running(docker, name, window_secs) -> bool` (ventana + RestartCount + HEALTHCHECK).
- [x] 2.3.2 `GuardOutcome { Updated, RolledBack, FailedStopped }` y `apply_with_guard(...)`:
      recrear → verificar; reintentar hasta N; rollback por digest; verificar post-rollback; parar si falla.
- [x] 2.3.3 Retirar `tag_backup_image` y el rollback por tag; unificar con el rollback inline de
      `recreate_container` (D1).

### 2.4 Integración en el worker/handlers
- [x] 2.4.1 `handlers.rs`: sustituir `verify_container_healthy` por `verify_stable_running` en
      `PullRestart` (`:296-310`) y `PullRestartStack` (`:404-418`).
- [x] 2.4.2 `handlers.rs`: usar `GuardOutcome` para decidir limpieza de imagen antigua (solo si sano)
      y para `tally_apply_outcome` (rollback sano cuenta como actualización fallida/rollback, no skips).
- [x] 2.4.3 `scheduler.rs`: resolver `max_start_attempts`/ventana por política con fallback global.

### 2.5 Notificaciones
- [x] 2.5.1 `notifications.rs`: mapeo explícito de estados de guarda (`rolled-back → warning`,
      `failed-stopped → critical`) sin romper los existentes.

### 2.6 API
- [x] 2.6.1 `admin.rs`: `get/put_update_check_config_h` con los 3 campos; `/api/update-policies` con overrides.

### 2.7 Frontend
- [x] 2.7.1 `types.ts`: ampliar `UpdateCheckConfig`, `UpdatePolicy`, `DefaultUpdatePolicy`, `HistoryEntry`.
- [x] 2.7.2 `ConfigPage.tsx`: inputs de los settings globales y overrides por política.
- [x] 2.7.3 `HistoryPage.tsx`: `formatStatus` con `rolled-back`/`failed-stopped` (+ `update-check-stack`),
      color/tooltip y columna de intentos.

## 3. REFACTOR y verificación

- [x] 3.1 `cd backend && just check` (fmt + clippy, cero warnings).
- [x] 3.2 `cd backend && cargo test` en verde.
- [x] 3.3 `cd frontend && pnpm run build` y `pnpm exec tsc -b`.
- [x] 3.4 `cd frontend && pnpm test` (vitest) en verde.
- [x] 3.5 `cd frontend && pnpm run lint`.

## 4. Cierre

- [x] 4.1 Auditar: `@rust-reviewer` (backend) y `@react-reviewer` (frontend).
- [x] 4.2 PR a `develop` (gitmoji), CI verde.
- [x] 4.3 `docs/update-rollback-guard.md` y `openspec archive update-rollback-guard` (+ limpieza del dir del change).

## 5. Cierre del gap D9 (supresión tras failed-stopped)

- [x] 5.1 RED: test de que un contenedor en `terminal_failed` no genera decisión de update aunque `has_update=true`.
- [x] 5.2 GREEN: `TerminalFailed` en `AppState`; inserción en `FailedStopped`; skip en el ciclo; limpieza en `has_update=false` y en `update_container_h`.
- [x] 5.3 Tests de helpers puros (`is_terminal_suppressed`, `record_terminal_failed`, `retain_present_terminal`).
