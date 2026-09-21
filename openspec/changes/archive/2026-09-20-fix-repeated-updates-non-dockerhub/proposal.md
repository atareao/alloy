# Proposal

## Why

Los contenedores de registros no-DockerHub (ECR, GHCR, Forgejo) se actualizan repetidamente de la misma versión antigua a la misma versión nueva en cada ciclo de check, porque el `image_id` que Docker asigna tras recrear con `image@manifest_digest` nunca coincide con el `config_digest` del registro. Esto genera entradas de historial falsas y pulls innecesarios.

## What Changes

- **Unificar la estrategia de comparación de digests** en los tres pathways de update (API single container, API check-all con policy, scheduler) para que usen `last_remote_digest` persistido en SQLite como fuente de verdad.
- **Persistir `last_remote_digest`** tras cada update exitoso en TODOS los pathways (actualmente solo el scheduler y `apply_single_policy` lo hacen).
- **Inicializar `last_remote_digest`** desde `image_id` en el primer ciclo si la BD está vacía.
- **No registrar entrada en el historial** si el digest remoto no ha cambiado respecto al `last_remote_digest`.

## Capabilities

### New Capabilities
- `updates/digest-comparison`: Lógica unificada de comparación de digests que evita falsos positivos en registros no-DockerHub, con persistencia de `last_remote_digest` en SQLite.

### Modified Capabilities
- Ninguna. No existen specs previas para el módulo de updates.

## Impact

- **`backend/src/updates/handlers.rs`**: Modificar `update_container_h`, `check_and_apply_all`, `apply_single_policy` para usar `last_remote_digest` y persistirlo tras cada update.
- **`backend/src/updates/common.rs`**: Añadir función `needs_update()` que centralice la comparación.
- **`backend/src/workers/scheduler.rs`**: Ya usa `last_remote_digest` correctamente, pero se beneficiará de la función centralizada.
- **`backend/src/db.rs`**: La tabla `container_policies` ya tiene columna `last_remote_digest` — verificar que se usa correctamente.
- **SQLite schema**: Sin cambios (columna `last_remote_digest` ya existe en `container_policies`).