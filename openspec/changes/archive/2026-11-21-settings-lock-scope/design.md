# Design — settings-lock-scope

## Contexto

`AppState` expone `Settings` como `Arc<tokio::sync::Mutex<Settings>>` y `DbPool` como
`deadpool_sqlite::Pool`. Los handlers de persistencia seguían este patrón:

```rust
let mut s = settings.lock().await;   // guard vivo
// ... mutar s ...
let conn = db_pool.get().await.unwrap();      // <-- await CON el guard
let _ = db::save_settings(&conn.lock().unwrap(), &s);
```

`db_pool.get().await` puede tardar (pool saturada). Con el guard vivo, cualquier otra tarea que
quiera `settings.lock()` queda encolada detrás de la espera de BD.

## Decisión

**Orden uniforme: conexión antes que estado.** En cada handler afectado se adquiere primero el
`Object` de la pool (`db_pool.get().await`), y sólo después se toma `settings.lock().await` para
mutar y persistir. El `conn.lock().unwrap()` (guard síncrono de `rusqlite`) se toma **dentro** de
la sección crítica, justo antes de `save_settings`, no antes:

```rust
let conn = db_pool.get().await.unwrap();   // espera la pool SIN ningún guard de Settings
let mut s = settings.lock().await;         // ahora sí, el mutex de estado
// ... mutar s ...
let _ = db::save_settings(&conn.lock().unwrap(), &s);  // guard std síncrono, dentro de la sección
drop(conn);                                // no retener el permiso de pool más de lo necesario
```

Por qué este orden y no "soltar el guard y clonar":

- **Sin `await` con guard de estado**: la sección crítica (mutar `s` + `save_settings`) es
  síncrona; el único `await` con estado ya no depende de la pool.
- **Sin ciclos de espera** entre pool y `Settings`: todos los handlers afectados adquieren en el
  mismo orden (pool → settings).
- **Menos allocaciones**: evita clonar `Settings` (struct grande) en cada PUT.

En el código final, el guard de `rusqlite` (`conn.lock()`) se toma dentro de la sección crítica
(no antes de `settings.lock()`): funcionalmente equivalente y con menos tiempo retenido del
`Mutex` estándar.

## Handlers afectados

| Handler | Fichero | Cambio |
|---|---|---|
| `update_config_h` | `backend/src/config.rs` | conn antes del `settings.lock()`; `drop(conn)` antes de `config_handler(...)` |
| `put_update_check_config_h` | `backend/src/admin.rs` | conn antes del `settings.lock()` |
| `put_default_update_policy_h` | `backend/src/admin.rs` | conn antes del `settings.lock()` |
| `export_config_h` | `backend/src/admin.rs` | conn antes de los locks (settings + history) |
| `import_config_h` | `backend/src/admin.rs` | conn antes del `settings.lock()` |

`export_config_h` mantiene su semántica actual (persistir *settings* antes de redactar la
respuesta); sólo se reordena la adquisición de recursos.

## Fuera de alcance (follow-up)

El orden *estado → pool* frente al mutex de `update_history` en `backend/src/updates/handlers.rs`
(`update_container_h`, `update_all_h`, `apply_single_policy`) NO se toca aquí: requiere su propio
change proposal. Issue **#170**. El requisito de esta capability se acota a los handlers que
persisten `Settings`.

## Alternativas descartadas

1. **Soltar el guard y clonar `Settings` para persistir**: correcto pero clona una struct grande
   en cada PUT y complica la atomicidad (la persistencia podría escribir un snapshot obsoleto si
   otra tarea muta entre el drop y el save).
2. **No persistir en el handler (delegar a un worker)**: cambio de arquitectura desproporcionado
   para el alcance del issue #158.

## Verificación

- Tests RED deterministas: pool de tamaño 1 ocupada por el test; se lanza el handler en una tarea
  y se comprueba, con `tokio::time::timeout`, que `settings.lock()` (y, en export, también
  `update_history.lock()`) sigue siendo adquirible mientras el handler espera conexión.
- Tests existentes de comportamiento (`config.rs`, `admin.rs`) deben seguir verdes.
- `cargo test`, `just check` (fmt + clippy) limpios.
