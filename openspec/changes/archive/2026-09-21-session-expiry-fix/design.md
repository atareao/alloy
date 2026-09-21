# Design: Session Expiry Enforcement in auth_me

## Approach

Modify the `auth_me` handler in `auth.rs` to validate session expiry (idle timeout and max duration) before returning `authenticated: true`. Currently it decodes the JWT and always returns success; the fix adds the same checks that `auth_middleware` already performs.

## Changes

### `auth.rs` — `auth_me` handler (lines 190-247)

**Before:**
```rust
let extract = |token: &str| -> Option<serde_json::Value> {
    jsonwebtoken::decode::<SessionClaims>(...)
        .ok()
        .map(|d| {
            let now = Utc::now().timestamp() as usize;
            let idle_remaining = ...;
            let session_remaining = ...;
            json!({ "authenticated": true, ... })
        })
};
```

**After:**
```rust
let extract = |token: &str| -> Result<serde_json::Value, Response> {
    let data = jsonwebtoken::decode::<SessionClaims>(...)
        .map_err(|_| unauthorized_json(false, "Invalid session token"))?;
    let claims = data.claims;
    let now = Utc::now().timestamp() as usize;

    // Check idle timeout
    let idle_elapsed = now.saturating_sub(claims.last_active);
    if idle_elapsed > idle_timeout_secs as usize {
        return Err(unauthorized_json(true, "Session expired: inactivity timeout. Please log in again."));
    }

    // Check max duration
    let session_elapsed = now.saturating_sub(claims.iat);
    if session_elapsed > max_duration_secs as usize {
        return Err(unauthorized_json(true, "Session expired: maximum duration reached. Please log in again."));
    }

    Ok(json!({ "authenticated": true, ... }))
};
```

The handler then maps `Err(response)` to return the 401 response directly.

### Response type change

The `extract` closure changes from returning `Option<Value>` to `Result<Value, Response>`. The cookie/header extraction loop changes from:
```rust
if let Some(resp) = extract(value) { return Json(resp); }
```
to:
```rust
match extract(value) {
    Ok(resp) => return Json(resp),
    Err(resp) => return resp,
}
```

### Reuse `unauthorized_json` helper

Already exists at line 274 — takes `(session_expired: bool, detail: &str) -> Response`. No new helper needed.

## Files Modified

- `backend/src/auth.rs` — `auth_me` handler only (~20 lines changed)

## No Frontend Changes

The frontend already:
- Checks `body.session_expired` in the periodic auth check (App.tsx:75)
- Redirects on 401 in `apiFetch` (api.ts:10-14)
- Checks `res.status === 401` in SSE error recovery (App.tsx:132)

All three paths will now trigger correctly when `auth_me` returns 401.