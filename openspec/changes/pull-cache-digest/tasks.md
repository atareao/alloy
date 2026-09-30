# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 `db.rs`: la migración añade `last_pulled_digest`; `update/get/load` funcionan
- [x] 1.2 `db.rs`: `load_last_pulled_digest_map` omite valores vacíos
- [x] 1.3 `handlers.rs`: helper puro `should_skip_pull(action, remote, last_pulled) -> bool`
- [x] 1.4 `handlers.rs`: `Pull` con digest igual → skip; distinto → pull
- [x] 1.5 `handlers.rs`: pull exitoso persiste `last_pulled_digest`; fallido no
- [x] 1.6 `handlers.rs`: `Pull` no limpia `has_update` ni avanza `last_remote_digest`
- [x] 1.7 Ejecutar `cargo test` y confirmar fallo de los nuevos tests con los legacy en verde

## 2. GREEN — Implementación mínima

- [x] 2.1 Migración `ALTER TABLE containers ADD COLUMN last_pulled_digest TEXT NOT NULL DEFAULT ''`
- [x] 2.2 `update_container_last_pulled_digest`, `get_container_last_pulled_digest`, `load_last_pulled_digest_map`
- [x] 2.3 `ContainerRow`/`ContainerInfo` con `last_pulled_digest`
- [x] 2.4 `PendingUpdate.last_pulled_digest` + carga en `check_and_apply_all`/`update_*_h`
- [x] 2.5 Rama `Pull` de `apply_single_policy`: skip si coincide; persistir tras éxito
- [x] 2.6 `cargo test` 100% verde + `cargo check`

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 `cargo fmt`
- [x] 3.2 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 `cargo test` sin regresiones
- [x] 3.4 Revisión con `rust-reviewer` (hallazgo del skip no-op corregido: `return;` + test `test_pull_skip_is_noop`)

## 4. Archive

- [ ] 4.1 `openspec validate pull-cache-digest --strict`
- [ ] 4.2 `openspec archive pull-cache-digest --yes`
- [ ] 4.3 Verificar/eliminar `openspec/changes/pull-cache-digest/`
