# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 `digest.rs`: parse de `gitea/gitea:1.27.3@sha256:87a6...` → `tag=1.27.3`, `digest=Some(...)`, `digest_only=false`
- [x] 1.2 `digest.rs`: parse de `nginx@sha256:abc...` → `tag=latest`, `digest=Some(...)`
- [x] 1.3 `digest.rs`: parse de `sha256:abc...` → `digest_only=true` (regresión)
- [x] 1.4 `digest.rs`: parse de `nginx:alpine` → `tag=alpine`, `digest=None` (regresión)
- [x] 1.5 `handlers.rs`: `recreate_container` produce `Config.Image=repo:tag` (sin `@sha256`)
- [x] 1.6 `handlers.rs`/`scheduler.rs`: el digest no avanza con política `None`/`Pull`; sí tras recreate (cubierto con helper puro `should_advance_digest`; integración real requiere Docker)
- [x] 1.7 `handlers.rs`: `PullRestartStack` persiste `last_remote_digest` (cubierto por lógica; integración real requiere Docker)
- [x] 1.8 Ejecutar `cargo test` y confirmar fallo de los nuevos tests con los legacy en verde

## 2. GREEN — Implementación mínima

- [x] 2.1 `ImageRef.digest: Option<String>` + `parse_image_ref` conserva tag real
- [x] 2.2 `check_remote_digest_impl` usa `parsed.tag` como reference
- [x] 2.3 `pull_image` retagüea `repo:tag` tras pull por digest (fuente canónica `repo@sha256:<d>`)
- [x] 2.4 `recreate_container` fija `Config.Image = tag_ref` (limpio)
- [x] 2.5 Quitar store incondicional en `check_and_apply_all`; store solo tras recreate
- [x] 2.6 Persistir digest tras `PullRestartStack` (scheduler + `apply_single_policy`) y `update_stack_h`
- [x] 2.7 Seed inicial: si `last_remote_digest` vacío, sembrar y comparar `image_id` vs remoto
- [x] 2.8 `cargo test` 100% verde (216 passed; 5 fallos de integración ambientales preexistentes) + `cargo check`

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 `cargo fmt`
- [x] 3.2 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 `cargo test` sin regresiones
- [x] 3.4 Revisión con `rust-reviewer` (M1/M2/M3/m4 corregidos)

## 4. Archive

- [x] 4.1 `openspec validate digest-pinned-refs --strict`
- [x] 4.2 `openspec archive digest-pinned-refs --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/digest-pinned-refs/`
