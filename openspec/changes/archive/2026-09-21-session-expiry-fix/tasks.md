# Tasks: Session Expiry Fix

## TDD Checklist

### RED — Write failing tests
- [ ] Test: `auth_me` returns `authenticated: true` for active session (within idle timeout and max duration)
- [ ] Test: `auth_me` returns 401 with `session_expired: true` when idle timeout exceeded
- [ ] Test: `auth_me` returns 401 with `session_expired: true` when max duration exceeded
- [ ] Test: `auth_me` returns `authenticated: false` for invalid/malformed token
- [ ] Test: `auth_me` returns `authenticated: false` when no cookie present

### GREEN — Implement minimal fix
- [ ] Modify `auth_me` handler to validate idle timeout and max duration
- [ ] Return 401 + `session_expired: true` when expired
- [ ] Return 200 + `authenticated: true` with session info when valid

### REFACTOR — Clean up
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Run `cargo fmt --check`
- [ ] Run `cargo test` — all tests pass

### Archive
- [ ] Run `openspec archive session-expiry-fix --yes`
- [ ] Verify `openspec/changes/session-expiry-fix/` is removed