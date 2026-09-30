# Proposal

## Why

Alloy pinnea cada contenedor al recrearlo tras un update: `recreate_container` fija
`Config.Image = repo:tag@sha256:<manifest>`. En el siguiente ciclo, `parse_image_ref`
convierte ese `@digest` en el literal `tag="digest"` y consulta
`.../manifests/digest` → HTTP 404 en todas las imágenes ya actualizadas. Peor aún: los
callers tratan el 404 como `has_update=false`, por lo que Alloy **deja de detectar
actualizaciones para siempre** en cada contenedor que ha actualizado una vez.

## What Changes

- `parse_image_ref` conserva el tag real y expone `digest: Option<String>`; deja de
  escribir `tag="digest"`.
- `check_remote_digest_*` resuelve el **tag** (`repo:tag`), nunca el pin ni el literal.
- `recreate_container` deja `Config.Image = repo:tag` (limpio); `pull_image` retagüea
  `repo:tag` tras pull por digest.
- `last_remote_digest` (config digest) es la fuente de verdad y **solo avanza tras un
  recreate exitoso**. Seed inicial desde `image_id` (sembrar y comparar).
- Los caminos de stack (`PullRestartStack`, `update_stack_h`) persisten el digest.
- Se elimina el guardado incondicional del digest en `check_and_apply_all`.

## Capabilities

### New Capabilities
- `updates/digest-pinned-refs`: parseo de referencias `tag@digest`, comprobación de
  updates por tag, `Config.Image` limpio al recrear y avance del digest solo al recrear.

### Modified Capabilities
<!-- Ninguna: updates/digest-comparison sigue vigente sin cambios de requisitos. -->

## Impact

- `backend/src/updates/digest.rs`: `ImageRef`, `parse_image_ref`, `check_remote_digest_impl`.
- `backend/src/updates/handlers.rs`: `recreate_container`, `check_and_apply_all`,
  `update_container_h`, `update_all_h`, `check_update_h`, `apply_single_policy`.
- `backend/src/workers/scheduler.rs`: check por tag y persistencia tras recreate/stack.
- `backend/src/containers.rs`: `pull_image` retagüea tras pull por digest.
- `backend/src/stacks.rs`: persistir digest tras `update_stack_h`.
- Sin cambios de frontend. Sin cambios de schema SQLite (`last_remote_digest` ya existe).

Fuera de alcance: los contenedores ya pinneados se auto-curan en su siguiente update
(no hay migración masiva). La política `Pull` seguirá reportando update sin avanzar el
digest.
