# PLAN-032 — Consistencia de `total` e `items` en `/api/history` (TOCTOU)

- **Issue:** #208
- **Change:** `openspec/changes/update-history-toctou/`
- **Rama:** `feature/update-history-toctou` (desde `develop`)
- **Fecha:** 2026-11-22

## Objetivo

Eliminar la lectura en dos transacciones implícitas de `GET /api/history`: `count_update_history` y `load_update_history_page` corrían como dos autocommits, así que un `append_update_history` concurrente podía commitear entre ambas y devolver un envelope incoherente.

## Alcance

- `backend/src/db.rs`: nueva `load_update_history_page_with_total(&mut Connection, ..)` con `transaction_with_behavior(Deferred)`; `count_update_history` y `load_update_history_page` pasan a privadas.
- `backend/src/updates/history.rs`: el handler usa la función única.
- Tests: página completa coherente, página más allá del final, y snapshot bajo appends concurrentes (DB en fichero, pool de 2, hilo writer).
- Documentación: follow-up del TOCTOU marcado como resuelto en `docs/update-history-pagination.md` (misma feature, sin fichero nuevo).
- Sin cambios de contrato HTTP, esquema ni frontend.

## Pasos

1. [x] Issue #208 y change proposal `update-history-toctou` validado.
2. [x] RED: stub con dos consultas + tres tests; el de concurrencia falla 3/3 con pares `(total, items.len())` incoherentes.
3. [x] GREEN: transacción `DEFERRED` con `commit()`; suite 590 passed; concurrencia 3/3.
4. [x] Auditoría `@rust-reviewer`: APROBADO, con test de mutación (revertir la transacción rompe el test 3/3).
5. [x] REFACTOR: funciones auxiliares privadas, docstring de `AppError::from_db` corregido, aserción del test endurecida.
6. [ ] Commit gitmoji → archivar change → PR a `develop` → cerrar #208 a mano.

## Verificación

```bash
cd backend && cargo test            # 590 passed
cargo test db::tests::test_page_with_total_is_snapshot   # 3/3
just check                          # fmt + clippy -D warnings
openspec validate --strict --specs
```

## Follow-ups

- **WAL y `busy_timeout`**: `init_db` ejecuta `PRAGMA journal_mode=WAL` e ignora el modo resultante; en un FS sin soporte WAL la lectura transaccional retendría SHARED y los `append_update_history` best-effort podrían perderse con `SQLITE_BUSY` (no hay `busy_timeout`). Candidato a change propio.
- Poda/retención de `update_history` y el resto de deudas de #197 siguen abiertos.