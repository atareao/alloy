# Orden de locks de `update_history` en helpers de historial

Change: `update-history-lock-order` · Issue #170 · Plan `plans/PLAN-025.md`

## Qué es

Issue #170. El issue apuntaba a `handlers.rs`, pero el análisis mostró que ese fichero ya estaba resuelto; el riesgo real estaba en `stacks.rs::append_stack_history`, `history.rs::delete_history_h` y el código muerto `common.rs::record_update_entry`. Los tres retenían el guard de `update_history` mientras esperaban una conexión de la pool, lo que podía cerrar un ciclo de espera con `export_config_h`/`import_config_h` (que van en orden `pool → estado`).

## Ficheros implicados

- `backend/src/stacks.rs` — `append_stack_history` suelta el guard antes de la pool.
- `backend/src/updates/history.rs` — `delete_history_h` suelta el guard antes de la pool.
- `backend/src/updates/common.rs` — se elimina `record_update_entry` (código muerto).
- `backend/src/test_support.rs` — helper de test de no-bloqueo con pool saturada.

## Cómo funciona

- **Patrón `estado → drop → pool`:** el cambio se publica en memoria dentro de la sección crítica, se suelta el guard y solo después se espera la conexión de la pool. Así el guard nunca cruza el `await` de la pool y se rompe el ciclo de espera con `export_config_h`/`import_config_h` (que van `pool → estado`).
- **`handlers.rs` no se tocó:** ya usaba `push_history_entry` con este patrón y cumplía.
- **Persistencia best-effort:** si falla la pool, la entrada permanece en memoria y se registra un `warn`.

## Variables/config

Ninguna.

## Cómo verificar

```bash
cd backend && cargo test        # 541 tests
cd backend && just check
```

## Decisiones

- No tocar `handlers.rs` porque ya cumplía.
- Eliminar `record_update_entry` en vez de arreglarlo por ser código muerto sin call-sites.
- Helper de test `assert_update_history_lock_free_while_db_pool_saturated` con pool `max_size(1)` saturada.

## Follow-ups

Ninguno.
