# Design: Fix Repeated Updates for Non-DockerHub Registries

## Problem

Tres pathways de update usan diferentes estrategias de comparación de digests, y ninguna maneja correctamente el caso de registros no-DockerHub donde el `image_id` de Docker (manifest digest tras recreate con `image@manifest_digest`) nunca coincide con el `config_digest` del registro.

## Solution

Centralizar la lógica de comparación en una función `needs_update()` que use `last_remote_digest` (persistido en SQLite) como fuente de verdad, con `image_id` como fallback solo en primera ejecución.

## Changes

### 1. `backend/src/updates/common.rs` — Añadir `needs_update()`

```rust
/// Determina si un contenedor necesita actualización comparando el digest local
/// (last_remote_digest de BD, o image_id como fallback) con el config_digest remoto.
///
/// Usa short_digest (12 chars) para la comparación.
/// Retorna `true` si los digests son diferentes o si local_digest está vacío.
pub fn needs_update(local_digest: &str, remote_digest: &str) -> bool {
    if local_digest.is_empty() {
        return true;
    }
    let local_short = crate::updates::digest::short_digest(local_digest);
    let remote_short = crate::updates::digest::short_digest(remote_digest);
    local_short != remote_short
}
```

### 2. `backend/src/updates/handlers.rs` — `update_container_h`

**Cambio**: En lugar de comparar `image_id` vs `config_digest`, cargar `last_remote_digest` de BD y usarlo para la comparación. Tras el update exitoso, persistir `last_remote_digest`.

```rust
// Antes (líneas 187-193):
let local_short = crate::updates::digest::short_digest(&image_id);
let remote_short = crate::updates::digest::short_digest(&config);
(local_short != remote_short, config, manifest)

// Después:
let last_remote = {
    let conn = db_pool.get().await.unwrap();
    db::get_container_last_remote_digest(&conn.lock().unwrap(), &name)
        .unwrap_or_else(|| image_id.clone())
};
let needs_pull = crate::updates::common::needs_update(&last_remote, &config);
```

Y tras el éxito (línea 285-301), añadir:
```rust
{
    let conn = db_pool.get().await.unwrap();
    let _ = db::update_container_last_remote_digest(&conn.lock().unwrap(), &name, &remote_digest);
}
```

### 3. `backend/src/updates/handlers.rs` — `check_and_apply_all`

**Cambio**: Cargar `last_remote_digest_map` al inicio (como hace el scheduler) y usarlo para la comparación en lugar de `image_id`.

```rust
// Al inicio de la función, cargar last_remote_digest_map:
let last_remote_digest_map = {
    let conn = db_pool.get().await.unwrap();
    db::load_last_remote_digest_map(&conn.lock().unwrap())
};

// En la comparación (línea 639), cambiar de:
let has_update = !image_id.is_empty() && image_id != config_digest;
// a:
let local_ref = last_remote_digest_map
    .get(&name)
    .map(|s| s.as_str())
    .unwrap_or(&image_id);
let has_update = crate::updates::common::needs_update(local_ref, &config_digest);
```

### 4. `backend/src/db.rs` — Añadir `get_container_last_remote_digest()`

```rust
pub fn get_container_last_remote_digest(conn: &Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT last_remote_digest FROM containers WHERE name = ?1",
        params![name],
        |row| row.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .filter(|s| !s.is_empty())
}
```

### 5. `backend/src/workers/scheduler.rs` — Usar `needs_update()` centralizada

Reemplazar la comparación inline (líneas 171-174) por la llamada a `crate::updates::common::needs_update()`.

## Files Modified

| File | Change |
|---|---|
| `backend/src/updates/common.rs` | Añadir `needs_update()` |
| `backend/src/updates/handlers.rs` | `update_container_h`: usar `last_remote_digest` + persistir tras éxito. `check_and_apply_all`: cargar `last_remote_digest_map` y usarlo. |
| `backend/src/db.rs` | Añadir `get_container_last_remote_digest()` |
| `backend/src/workers/scheduler.rs` | Usar `needs_update()` centralizada |

## No Changes Needed

- `apply_single_policy` (handlers.rs:1233-1247): ya persiste `last_remote_digest` correctamente.
- `load_last_remote_digest_map` (scheduler.rs:581): ya existe en db.rs.
- SQLite schema: columna `last_remote_digest` ya existe en `container_policies` (o `containers`).

## Test Strategy

1. **Unit test**: `needs_update()` con digests iguales, diferentes, vacío.
2. **Integration test**: Simular escenario donde `image_id` es manifest digest (no coincide con config_digest) pero `last_remote_digest` sí coincide → `needs_update()` retorna `false`.
3. **Integration test**: Verificar que `update_container_h` persiste `last_remote_digest` tras éxito.
4. **Integration test**: Verificar que `check_and_apply_all` usa `last_remote_digest_map` en lugar de `image_id`.