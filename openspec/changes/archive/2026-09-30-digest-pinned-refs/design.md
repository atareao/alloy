# Design

## Context

El pipeline de updates tiene tres pathways (API single, check-all, scheduler) más dos
caminos de stack. Todos comparan el digest remoto contra `last_remote_digest` (config
digest) persistido en SQLite, con `image_id` como fallback. El check remoto parte de
`container.image` (`Config.Image`), que Alloy reescribe a `repo:tag@sha256:<manifest>` al
recrear. `parse_image_ref` colapsa ese `@digest` a `tag="digest"` y la consulta al
registro falla con 404.

## Goals / Non-Goals

**Goals:**
- Que el check de update resuelva siempre el tag subyacente y nunca dé 404 por un pin.
- Que `Config.Image` quede limpio (`repo:tag`) tras un update.
- Que `last_remote_digest` represente la imagen en ejecución y solo avance al recrear.
- Persistir el digest en los caminos de stack.

**Non-Goals:**
- Migración masiva de contenedores ya pinneados (se auto-curan en su siguiente update).
- Cambiar el schema SQLite (`last_remote_digest` ya existe).
- Cambios de frontend.
- Evitar pulls repetidos en política `Pull` (se documenta como comportamiento conocido).

## Decisions

### D1: `ImageRef` con `digest` separado y tag real
`parse_image_ref` deja de escribir `tag="digest"`; guarda el tag real y el digest en
`digest: Option<String>`. Alternativa descartada: mantener `tag="digest"` y añadir un
campo aparte — deja la UI mostrando `digest` y complica el check.

### D2: El check usa `parsed.tag` como reference
`check_remote_digest_impl` pasa `parsed.tag` a `fetch_manifest_digests`. Para
`repo:tag@sha256:...` consulta `repo:tag`; para `repo:tag` normal, igual que hoy. El
`digest_only` (`sha256:...` puro) sigue devolviendo `Err`.

### D3: Pull por digest + retag, recreate por tag
`pull_image` mantiene el pull por digest (contenido exacto) pero retagüea `repo:tag` con
`docker.tag_image` tras el pull. `recreate_container` fija `Config.Image = tag_ref`
(sin `@digest`). Alternativa descartada: pull por tag directo — reintroduce la ventana de
caché de tag que motivó el pull por digest.

### D4: `last_remote_digest` = config digest, avanza solo al recrear
Se elimina el `update_container_last_remote_digest` incondicional de
`check_and_apply_all`. El digest se persiste únicamente tras recreate exitoso
(`update_container_h`, `update_all_h`, `apply_single_policy` PullRestart, scheduler
PullRestart) y tras actualización de stack. Seed: si está vacío, se siembra y se compara
`image_id` vs remoto (detecta updates en contenedores cegados).

### D5: Persistencia en caminos de stack
Tras `docker compose pull && up -d`, resolver el config digest local del servicio
(inspeccionando la imagen del contenedor recreado) y persistirlo.

## Risks / Trade-offs

- **Política `Pull` sin recreate**: seguirá reportando update y hará pull cada ciclo.
  Aceptado; se puede añadir `last_pulled_digest` en un cambio futuro.
- **Primer ciclo con seed+comparar**: puede marcar update en contenedores obsoletos y
  recrearlos. Es el comportamiento deseado (recuperación), pero implica un recreate
  inicial en contenedores que llevaban tiempo cegados.
- **Retag tras pull por digest**: si el retag falla, el recreate por tag podría usar una
  imagen cacheada. Se trata el fallo como error del update (no se recrea).
- **Carrera tag/pull**: entre el check y el pull el tag podría moverse; el pull por
  digest exacto lo evita, y el digest persistido es el del contenido realmente descargado.
