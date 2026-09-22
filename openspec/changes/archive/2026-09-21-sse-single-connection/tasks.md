# Tasks: Single SSE Connection

## Backend
- [ ] Add `/api/stream` endpoint that merges all three broadcast channels (StateEvent, UpdateProgress, NotifEvent) into a single SSE stream, dispatching events by type (`containers`, `update-progress`, `notification`)
- [ ] Register `/api/stream` route in `main.rs`

## Frontend
- [ ] Replace 3 separate SSE `useEffect` hooks in `App.tsx` with a single connection to `/api/stream`
- [ ] Listen for `containers`, `update-progress`, and `notification` events on the single connection
- [ ] Remove unused `useSSE` hook or update it

## Auth
- [ ] Ensure `/api/stream` is protected by auth middleware (it already is, no changes needed)
- [ ] Keep the existing SSE header-skip fix in `auth_middleware`

## Tests
- [ ] Update frontend tests if any reference SSE connections
- [ ] Verify backend tests still pass