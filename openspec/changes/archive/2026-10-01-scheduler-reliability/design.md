# Design

## Context

El worker `update-check` corre en `tokio::spawn`. Sus helpers de persistencia usan
`unwrap()` sobre el pool y los mutex; un fallo transitorio puede paniquear y matar la tarea
sin reinicio. Además, en modo degradado la política `Pull` no tiene anti-bucle y re-descarga
cada ciclo.

## Goals / Non-Goals

**Goals:**
- Cero `unwrap()` sobre pool/mutex en los helpers del worker.
- No repetir el pull degradado del mismo manifest.

**Non-Goals:**
- Persistir `last_remote_digest` en `Pull` (la semántica de no avanzar se mantiene).
- Cambiar la lógica de comparación (`needs_update`).
- NITs de `stacks.rs` ni clones de `resolve_digest_cached`.

## Decisions

### D1: Manejo explícito en helpers
Cada helper pasa de `unwrap()` a `if let Ok(conn) = db_pool.get().await { match conn.lock() { Ok(g) => ..., Err(e) => tracing::error!(...) } } else { tracing::error!(...) }`, replicando el patrón ya usado por `persist_last_remote_digest_after_recreate`.

### D2: Guard degradado compartido por `Pull`
Se reutiliza `degraded_applied` (mapa nombre→manifest) y el helper puro
`degraded_update_suppressed`. En la rama `Pull`, si el manifest degradado ya se aplicó, se
suprime el pull; si se aplica, se inserta; si la resolución es fiable, se limpia. Sin
persistir `last_remote_digest`.

### D3: Log de persistencia degradada
El segundo `WARN` de `persist_last_remote_digest_after_recreate` baja a `info!`; el único
`WARN` de desenlace degradado sigue siendo el de `classify_digest_outcome`.

## Risks / Trade-offs

- [Riesgo] El test de forma (grep-asert) puede ser frágil ante refactors. Mitigación: acotarlo
  a los patrones `db_pool.get().await.unwrap()` y `lock().unwrap()` en el archivo del worker.
- [Riesgo] El guard en memoria de `Pull` no sobrevive reinicios; tras reiniciar podría
  repetirse un pull degradado. Aceptable (se autocorrige al recuperarse el registry).

## Migration Plan

Sin migración de datos. Rollback = revertir commit.

## Open Questions

- ¿Incluir también los NITs de `stacks.rs` en este change o dejarlos para otro? (por
  defecto: fuera de alcance).
