# Fix TypeScript Build Error in App.test.tsx

## Why
The Docker build (`pnpm run build`) fails with a TypeScript compilation error in `frontend/src/App.test.tsx:211`, blocking all image builds and deployments.

## What Changes
Replace the `delete window.location` + direct reassignment pattern with `Object.defineProperty` in the logout test, which avoids the `Type 'Location' is not assignable to type 'string & Location'` type error.

## Scope
Single file change: `frontend/src/App.test.tsx`, lines 191-195 and 211.

## Impact
- Unblocks Docker image builds
- No runtime behavior changes
- No API or backend changes

## Root Cause
On line 211, `window.location = originalLocation` attempts to restore the original `Location` object after a test mock. After `delete window.location` and reassignment to `{ href: "" }`, TypeScript's internal type for `window.location` becomes `string & Location`, making `Location` unassignable back.

## Fix Strategy
Replace the `delete`/reassign pattern with `Object.defineProperty` for both the mock and the restore, which avoids the type conflict entirely.