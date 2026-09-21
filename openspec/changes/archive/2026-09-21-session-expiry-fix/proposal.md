# Proposal: Session Expiry Enforcement in auth_me

## Why

The `/api/auth/me` endpoint is the frontend's sole mechanism for detecting session expiry (used on page load, every 5 minutes, and on SSE error recovery). However, it only decodes the JWT and reports remaining time — it never rejects an expired session. This means a user can leave the browser open overnight and find the dashboard still active the next morning, because the SSE connections stay alive and the periodic auth check always returns `authenticated: true`.

## What Changes

- **`auth_me` handler** (`auth.rs`): Add idle timeout and max duration validation before returning `authenticated: true`. If either check fails, return HTTP 401 with `session_expired: true` instead of a 200 with `authenticated: true`.
- **No frontend changes needed**: The frontend already handles `body.session_expired` in the periodic check (App.tsx:75) and `apiFetch` already redirects on 401 (api.ts:10-14).

## Capabilities

### New Capabilities
- `auth/session`: Session lifecycle management — creation, validation, idle timeout enforcement, max duration enforcement, and expiry detection via `/api/auth/me`.

### Modified Capabilities
- None. No existing spec covers auth/session behavior.

## Impact

- **Backend**: Only `auth.rs` — the `auth_me` handler function (~60 lines). No new dependencies.
- **Frontend**: No changes needed. The existing `session_expired` handling in App.tsx and `apiFetch` already works correctly; it just never receives the signal.
- **Tests**: New tests in `auth.rs` for `auth_me` with expired idle timeout and expired max duration.