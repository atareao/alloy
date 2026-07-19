# PLAN: Alloy — Limpieza y Refactor

## Objetivo
Eliminar deuda técnica identificada en la revisión: singletons, mutex global, código duplicado, componentes monolíticos, estilos inline.

## 1. Backend — db::global() singleton y Mutex global

### Problema
- `db::global()` es un `OnceLock<DbPool>` usado 31 veces como singleton estático.
- `DbPool = Arc<Mutex<Connection>>` serializa todo el acceso a SQLite.
- Los handlers que tienen acceso a `AppState` siguen usando `db::global()` en vez de `State(db_pool)`.

### Solución
1. **Añadir `deadpool-sqlite`** como pool de conexiones async-native.
   - `DbPool = deadpool_sqlite::Pool`
   - Conexiones WAL-mode, acceso concurrente real.
2. **Eliminar `db::global()`**: pasar `DbPool` explícitamente:
   - Handlers: `State(db_pool): State<DbPool>` (ya hay `FromRef`).
   - Workers: añadir `db_pool: DbPool` como parámetro en `tokio::spawn()`.
   - `db.rs`: exportar funciones que tomen `&Connection` (ya lo hacen).
3. **Consolidar `init_db`/`init_test_db`** y eliminar `init_global()`.

### Archivos afectados
- `backend/src/db.rs` — nuevo pool, eliminar `global()`, `init_global()`
- `backend/src/main.rs` — crear pool, pasar a workers
- `backend/src/containers.rs` (2 sites) — recibir db_pool vía State
- `backend/src/config.rs` (1 site)
- `backend/src/admin.rs` (6 sites)
- `backend/src/updates/handlers.rs` (16 sites)
- `backend/src/updates/history.rs` (1 site)
- `backend/src/workers/auto_update.rs` (2 sites)
- `backend/src/workers/scheduler.rs` (1 site)

---

## 2. Backend — Código duplicado en updates

### Problema
Los handlers `update_container_h`, `update_all_h`, `apply_policies_background`, y `update_check_worker` comparten el mismo patrón:
1. set_updating
2. pull_image
3. restart_container
4. Crear UpdateHistoryEntry
5. append_update_history
6. clear_updating
7. notify_all + notif_tx.send
8. update_container_has_update(false)

Cada bloque se repite ~10 veces con pequeñas variaciones.

### Solución
Crear helpers en `updates/handlers.rs` (o nuevo `updates/common.rs`):
- `record_update_entry(db, name, image, status, duration_ms) -> UpdateHistoryEntry`
- `notify_update_complete(notif_tx, settings, name)`
- `mark_update_done(db, name)` — set has_update=false, clear_updating

---

## 3. Backend — Auto-update worker con cron configurable

### Problema
`auto_update_worker` usa `hours = 6` hardcodeado (línea 24 de `auto_update.rs`).

### Solución
- En cada tick del loop, leer `auto_update_interval_hours` de Settings.
- Re-crear el interval si cambió.
- O mejor: usar el mismo `update_check_cron` de Settings y reutilizar la lógica.

Implementación: leer settings al inicio del tick, si `auto_update_enabled` es false, hacer `continue`. Si el intervalo cambió, recrear el timer.

---

## 4. Frontend — Dividir DashboardPage.tsx (1.571 → ~300 líneas)

### Problema
DashboardPage.tsx tiene 1.571 líneas con múltiples responsabilidades.

### Solución
Extraer componentes:

| Componente | Líneas estimadas | Responsabilidad |
|---|---|---|
| `ContainerTable.tsx` | ~200 | Tabla/grupos de contenedores, búsqueda, filtros |
| `ContainerRow.tsx` | ~100 | Fila individual con acciones |
| `ContainerActions.tsx` | ~80 | Botones de acción por contenedor |
| `InspectModal.tsx` | ~120 | Modal de inspección detallada |
| `LogsModal.tsx` | ~100 | Modal de logs en vivo |
| `BatchProgress.tsx` | ~80 | Barra de progreso de check/update batch |
| `SummaryDialog.tsx` | ~60 | Diálogo de resumen de resultados |
| DashboardPage.tsx (refactor) | ~200 | Orquestación, estado compartido, layout |

---

## 5. Frontend — showNotification + Mantine Styles

### Problema
- Sistema dual de notificaciones: `showNotification` de Mantine + toasts manuales con estilos inline.
- Estilos inline en notificaciones toasts y botón "Limpiar todas".

### Solución
- Eliminar el sistema de toasts manual en App.tsx y NotifToast.tsx.
- Usar `showNotification` de `@mantine/notifications` en todos los casos.
- Reemplazar estilos inline con props de Mantine (style, className, Paper, Group, etc.)

---

## 6. Frontend — Limpiar páginas inexistentes

### Problema
CodeGraph index muestra referencias a `SchedulePage.tsx`, `StacksPage.tsx`, `TerminalPage.tsx`, `HealthChecksPage.tsx`, `AlertsPage.tsx` pero no existen en disco.

### Solución
- No hay nada que limpiar (no existen en disco). Solo verificar que `App.tsx` no tenga imports obsoletos.

---

## Flujo de ejecución

```
FASE 1 (Backend): rust-dev
  └─ deadpool-sqlite + eliminar db::global()
  └─ Helpers de update (common.rs)
  └─ Auto-update cron configurable

FASE 2 (Frontend): frontend-dev
  └─ Dividir DashboardPage.tsx
  └─ Unificar notificaciones (showNotification)
  └─ Mantine Styles en vez de inline

FASE 3 (Verificación): code-reviewer + cargo test + npm test
```
