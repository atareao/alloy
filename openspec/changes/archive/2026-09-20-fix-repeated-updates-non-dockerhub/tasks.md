# Tasks: Fix Repeated Updates for Non-DockerHub Registries

## TDD Checklist

### RED — Write failing tests

- [ ] **T1**: Unit test `needs_update()` — same digests → false
- [ ] **T2**: Unit test `needs_update()` — different digests → true
- [ ] **T3**: Unit test `needs_update()` — empty local digest → true
- [ ] **T4**: Unit test `needs_update()` — manifest digest (image_id) != config_digest, but last_remote_digest == config_digest → false
- [ ] **T5**: Integration test — `update_container_h` persists `last_remote_digest` after success
- [ ] **T6**: Integration test — `check_and_apply_all` uses `last_remote_digest_map` instead of `image_id`

### GREEN — Implement

- [ ] **G1**: Add `needs_update()` to `backend/src/updates/common.rs`
- [ ] **G2**: Add `get_container_last_remote_digest()` to `backend/src/db.rs`
- [ ] **G3**: Modify `update_container_h` to use `last_remote_digest` for comparison and persist after success
- [ ] **G4**: Modify `check_and_apply_all` to load `last_remote_digest_map` and use it for comparison
- [ ] **G5**: Modify scheduler to use `needs_update()` centralizada

### REFACTOR — Clean up

- [ ] **R1**: Run `cargo fmt --check`
- [ ] **R2**: Run `cargo clippy -- -D warnings`
- [ ] **R3**: Run `cargo test` — all tests pass
- [ ] **R4**: Run `cargo check` — no compilation errors

### ARCHIVE

- [ ] **A1**: Mark all tasks complete
- [ ] **A2**: Run `openspec archive fix-repeated-updates-non-dockerhub --yes`
- [ ] **A3**: Verify `openspec/changes/fix-repeated-updates-non-dockerhub/` was removed