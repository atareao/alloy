# Design — update-rollback-guard

## Contexto

Estado actual (mapa factual del worker):

- El sondeo de salud es `verify_container_healthy` (`updates/handlers.rs:1696`) con delays
  fijos `[3, 6, 12]` (= 21 s) y solo comprueba `state == "running"`, sin ventana estable ni
  `RestartCount` ni `HEALTHCHECK`.
- El rollback usa `tag_backup_image` (`handlers.rs:1728`) que crea el tag frágil
  `rollback-<YYYYMMDDHHMMSS>` y `rollback_container` (`handlers.rs:1747`) re-tag + restart +
  `remove_image(new_image)`. No verifica que la imagen anterior arranque. Si falla, solo loguea.
- Existe **otro** mecanismo de rollback inline en `recreate_container` (`handlers.rs:228-249`):
  rename `{name}-old-{unix_secs}` y restart. Duplicidad de mecanismos.
- `RestartCount` y `HealthConfig`/`HEALTHCHECK` **no se consumen**: solo `state.health.status`
  se lee (y solo en `inspect_container_h`, `containers.rs:547-551`), no en el worker.
- Niveles de notificación: `notify_all_state` → `valet_level` (`notifications.rs:238`):
  `critical` si contiene `died|dead|error`; `warning` si `exit|stop`; `info` resto. El
  scheduler usa `notify_all` (raw `None` ⇒ `info`).

## Objetivos

1. Detección fiable de "arrancó bien": running **estable** en una ventana + `RestartCount` sin
   subir + `healthy` si hay HEALTHCHECK.
2. Reintentos de arranque hasta N (default 3), global + override por política.
3. Rollback por **digest exacto** persistido; no eliminar la imagen anterior hasta confirmar la
   nueva sana.
4. Verificación post-rollback: `rolled-back` (warn) o `failed-stopped` (error + contenedor parado).
5. Persistencia (historial: digests, intentos, estado) y notificación del resultado.

## Non-goals

- Migraciones de datos entre versiones de imagen.
- Reintentos en el flujo de update **manual** que no pasa por el worker (se evalúa en este
  change: si el update manual usa el mismo `recreate_container`, hereda la guarda; no se añade
  UI específica).
- Cambiar defaults de cron/`pull_timeout_secs` (discrepancias detectadas, fuera de alcance).

## Decisiones

### D1 — Un único mecanismo de rollback (digest exacto)

Se **unifica** en la guarda: se retira el tag `rollback-<ts>` (`tag_backup_image`) y el rollback
inline de `recreate_container` pasa a delegar en la guarda. La imagen anterior se identifica por
su **`image_id`/digest exacto** (el `image_id` del contenedor antes de recrear, ya disponible en
los `PendingUpdate`/inspect). El rollback recrea con esa referencia exacta.

### D2 — Lectura de salud y `RestartCount` vía Bollard

Nuevo helper en `containers.rs` (p. ej. `inspect_runtime_health(docker, name)`) que devuelve:
`state` (`HealthStatus`→status string), `restart_count: u64` y `health: Option<String>`
(`Some` ⇒ la imagen define HEALTHCHECK). Se consume en la guarda, no solo en el handler HTTP.
`HealthStatus` enum de Bollard: `starting | healthy | unhealthy | none`.

### D3 — Ventana estable

La guarda muestrea N veces dentro de `update_health_window_secs` (default 30): el contenedor es
sano si en todas las muestras `state == running`, `restart_count` no aumenta respecto a la
primera muestra, y si `health == Some(_)` entonces nunca `unhealthy` y termina `healthy`.

### D4 — Reintentos

`max_start_attempts = update_max_start_attempts` (default 3), con
`update_start_attempt_interval_secs` entre intentos. Cada intento = volver a arrancar/recrear
la imagen nueva y re-verificar la ventana. Override por política (`max_start_attempts`) gana.

### D5 — Niveles de notificación

`valet_level` actual no mapea bien: `rolled-back` → `info` y `failed-stopped` → `warning`
(contiene `stop`). Se **amplía** el mapeo para los estados de la guarda de forma explícita:
`rolled-back → warning`, `failed-stopped → critical` (error). Se pasa el estado explícito a
`notify_all_state` (no depender de la heurística de substrings) para los estados nuevos.

### D6 — Historial

`UpdateHistoryEntry` (`models.rs:196`) ya tiene `old_digest`, `new_digest`, `status`. Se añade:
`attempts: u32` (serde default 0) y se documentan los nuevos valores de `status`:
`rolled-back`, `failed-stopped` (junto a los `update-check-*` existentes). Migración idempotente
en `db.rs` (`ALTER TABLE update_history ADD COLUMN attempts`).

### D7 — Settings y política

- `Settings` (`models.rs:207`): `update_max_start_attempts: u32` (default 3),
  `update_start_attempt_interval_secs: u64`, `update_health_window_secs: u64` — `serde(default)`.
- `UpdatePolicy` (`models.rs:256`) + `UpdatePolicyReq`: overrides `max_start_attempts: Option<u32>`,
  `health_window_secs: Option<u64>` — migraciones idempotentes de columnas en `update_policies`.
- `UpdateCheckConfig` (`models.rs:312`): expone los tres campos globales.

### D8 — Estados de "en curso" en la UI

`HistoryPage.formatStatus` no mapea `update-check-stack`; se aprovecha para mapear también los
estados nuevos (`rolled-back`, `failed-stopped`) con color/tooltip propios.

## API

- `GET/PUT /api/update-check/config`: + `update_max_start_attempts`,
  `update_start_attempt_interval_secs`, `update_health_window_secs`.
- `/api/update-policies` (PUT): acepta `max_start_attempts`, `health_window_secs`.
- `GET /api/history`: entradas incluyen `attempts` y los nuevos `status`.

## Frontend

- `ConfigPage.tsx`: campos numéricos para los 3 settings globales + override por política.
- `HistoryPage.tsx`: mapeo de estados nuevos + columna de intentos (tooltip/color).
- `types.ts`: ampliar `UpdateCheckConfig`, `UpdatePolicy`, `DefaultUpdatePolicy`, `HistoryEntry`.

## Riesgos

- **Doble lectura de estado** (worker vs handler HTTP): encapsular en un helper único.
- **Rendimiento**: la ventana añade espera por contenedor; el `check_interval_ms` (2000) debe
  seguir gobernando el ritmo del ciclo — la guarda solo corre al aplicar un update, no en el check.
- **Regresión de notificaciones**: cambiar el mapeo de niveles toca `notify_all_state`; cubrir
  con tests los estados nuevos sin alterar los existentes.
- **Contenedores de compose**: la guarda debe funcionar igual en `PullRestartStack` (recrear
  servicio) que en `PullRestart`.

## Alternativas descartadas

- Mantener el tag `rollback-<ts>`: frágil y no garantiza el digest exacto.
- `HEALTHCHECK` implícito por tiempo fijo: no cubre crash-loop sin healthcheck.
- Estado terminal como `exited` genérico: se prefiere `failed-stopped` explícito y accionable.

## D9 — Supresión de reintentos tras `failed-stopped`

El scenario «No hay reintentos infinitos tras el fallo terminal» exige que un contenedor en
`failed-stopped` NO se reintente en ciclos posteriores del worker hasta intervención explícita
del operador. Mecanismo (mínimo y testable):

- Estado en memoria del scheduler análogo a `degraded_applied_manifest`: un conjunto
  `terminal_failed: HashSet<String>` (por nombre de contenedor) en el estado de updates.
- Al alcanzar `GuardOutcome::FailedStopped`, se inserta el contenedor en `terminal_failed`.
- `compute_update_decision` (o el filtro previo del ciclo) MUST saltar los contenedores
  presentes en `terminal_failed` aunque `has_update` sea `true`.
- La entrada se limpia ante **intervención explícita**: update/start manual del contenedor
  (`update_container_h`) o al detectarse cambio de imagen/`has_update=false`.

Persistencia: en memoria en esta iteración (se pierde al reiniciar el proceso, igual que otras
cachés del worker). La persistencia en disco queda como follow-up si se requiere.

## D10 — Anti-churn tras `rolled-back`

Tras un `RolledBack`, el contenedor queda corriendo la imagen antigua con `has_update=true`, por lo
que el siguiente ciclo volvería a `pull` + N intentos (~90 s) + rollback + `warn`, indefinidamente.
Mecanismo (mismo patrón que D9 pero con clave de digest):
- Mapa `rolled_back: HashMap<String /*container*/, String /*attempted_digest*/>` en el estado de updates.
- Tras `RolledBack`, insertar `(name, new_digest)`.
- El ciclo MUST saltar el contenedor mientras el digest remoto resuelto coincida con el registrado;
  si el digest remoto cambia (nueva versión publicada), MUST reintentar.
- Limpieza ante intervención manual (`update_container_h`/`update_all_h`) o `has_update=false`.

## D11 — Los flujos manuales heredan la guarda

El scenario «Imagen antigua retenida hasta confirmar la nueva sana» es un MUST. Hoy
`update_container_h`, `update_all_h` y `update_stack_h` recrean y **eliminan la imagen antigua sin
verificación**. Se extrae un helper compartido `apply_with_guard` (recrear → `verify_stable_running`
→ reintentos → rollback por digest → terminal) y lo usan **tanto el worker como los handlers
manuales**, de modo que la imagen anterior solo se elimina tras confirmar la nueva sana. Esto
elimina además la duplicación del bloque decisión→historial→notificación (hallazgo 8 del reviewer).

## D12 — Error de inspección ≠ contenedor no sano

`verify_stable_running` devuelve `Result<bool, AppError>`. Un fallo de `inspect_container`
(Docker API) es **inconcluso**, no un «no arrancó sano». En la guarda, ante error de inspección
persistente se registra `ERROR` con la causa y se **omite el teardown** (ni rollback ni stop) y no
se elimina la imagen anterior, evitando rollbacks/paradas espurios por fallos transitorios.
