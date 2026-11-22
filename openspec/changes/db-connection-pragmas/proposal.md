# Proposal

## Why

`init_db` aplica `PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;` en **una única conexión** (la que crea la pool) y descarta el valor devuelto por el PRAGMA de journal. El resto de conexiones que abre `deadpool_sqlite` nace con los defaults de SQLite: `busy_timeout = 0` y `foreign_keys = OFF`. Tres consecuencias reales:

1. **Pérdida silenciosa de entradas de histórico.** Los appends son best-effort: `workers/scheduler.rs:1032` hace `let _ = db::append_update_history(...)`. Con `busy_timeout = 0`, cualquier colisión entre escritores (WAL admite un solo writer simultáneo) devuelve `SQLITE_BUSY` al instante y la entrada se pierde sin log.
2. **Degradación no detectada del modo journal.** Si el sistema de ficheros no soporta WAL, `PRAGMA journal_mode=WAL` devuelve `delete` **sin error**. Desde #208 la lectura de `/api/history` mantiene una transacción de lectura: en modo rollback eso retiene SHARED durante ambas consultas y agrava el punto 1, sin que nadie se entere.
3. **PRAGMAs inconsistentes entre conexiones.** `foreign_keys=ON` se aplica solo a la conexión de inicialización; el resto corre en OFF. Hoy no hay `REFERENCES` en el esquema, pero es una trampa para el futuro.

Issue #210.

## What Changes

- **PRAGMAs de sesión en cada conexión de la pool**, aplicados en la creación del `Manager` en vez de una sola vez en `init_db`: `busy_timeout = 5000` ms y `foreign_keys = ON`.
- **Verificación del modo journal en `init_db`**: se lee el valor devuelto por `PRAGMA journal_mode=WAL` y, si no es `wal`, se emite `tracing::warn!` accionable (qué modo quedó activo y qué implica) **sin abortar el arranque**, porque `busy_timeout` hace que el modo rollback siga siendo funcional, solo más contencioso.
- **Append del histórico no silencioso**: el `let _ =` de `workers/scheduler.rs` pasa a registrar el error con `tracing::error!` incluyendo contenedor e imagen. Los otros dos puntos de append (`updates/handlers.rs:522`, `stacks.rs:429`) ya loguean y no se tocan.
- Sin cambios de contrato HTTP, de esquema ni de frontend.

## Capabilities

### New Capabilities

- `backend/db-connection`: formaliza la configuración de las conexiones SQLite del backend (PRAGMAs de sesión aplicados a toda la pool y verificación del modo journal al arrancar).

### Modified Capabilities

Ninguna. `backend/update-history-pagination` no cambia: su requirement de instantánea sigue igual.

## Impact

- Código: `backend/src/db.rs` (construcción de la pool + `init_db`), `backend/src/workers/scheduler.rs` (log del append).
- **Riesgo de tipo de pool**: `deadpool_sqlite::Config` (0.13) no expone hooks `post_create`, así que los PRAGMAs por conexión requieren un `deadpool::managed::Manager` propio que envuelva al de `deadpool_sqlite` y aplique los PRAGMAs tras crear cada conexión. `DbPool` deja de ser `deadpool_sqlite::Pool` y pasa a `deadpool::managed::Pool<PragmaManager>`; los ~60 puntos que hacen `pool.get().await` no cambian porque la API de `Object` es la misma, y `AppError::from_pool` es genérico sobre `Display`, así que tampoco cambia.
- **Riesgo de comportamiento**: `foreign_keys=ON` en todas las conexiones podría hacer fallar escrituras hoy toleradas; hoy el esquema no declara `REFERENCES`, así que no hay cambio efectivo. `busy_timeout=5000` puede retrasar hasta 5 s una escritura bajo contención extrema en vez de fallar; los handlers afectados ya son asíncronos y no retienen el lock del histórico.
- Tests: los helpers `test_pool`, `test_pool_with_max_size` y `pool_with_schema` cambian de construcción pero mantienen firma y semántica.