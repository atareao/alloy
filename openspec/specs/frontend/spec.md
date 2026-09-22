# Spec Delta: Frontend State Polling

## ADDED

### New Hook: `useStatePoll`
```typescript
function useStatePoll(
  onContainers: (containers: ContainerInfo[]) => void,
  onError?: () => void,
): void
```

**Behavior**:
1. Calls `GET /api/state` with `credentials: "include"`
2. On success (200), parses JSON as `ContainerInfo[]` and calls `onContainers`
3. Immediately makes another request (no delay between requests)
4. On error (network failure, non-200 status), waits with exponential backoff (1s, 2s, 4s, ... up to 30s max) before retrying
5. Resets backoff to 0 on successful response
6. Cleans up on unmount (aborts in-flight request)

### New Progress Polling Effect in `App.tsx`
- `useEffect` that polls `GET /api/check-progress` every 2 seconds when `batchPhase === "active"`
- Handles `__batch__` done event to show summary and reset state

## REMOVED

### Removed Files
- `frontend/src/useWS.ts` — WebSocket hook
- `frontend/src/useWS.test.tsx` — WebSocket tests

### Removed from `App.tsx`
- `useWS("/api/ws", "containers", ...)` call
- `useWS("/api/ws", "notification", ...)` call
- `useWS("/api/ws", "update-progress", ...)` call

## MODIFIED

### `App.tsx`
- Replaced `import { useWS } from "./useWS"` with `import { useStatePoll } from "./useStatePoll"`
- Replaced three `useWS` calls with `useStatePoll` + progress polling `useEffect`

## Scenarios

### Happy Path: Continuous state updates
**Given** the frontend calls `useStatePoll`
**When** the first response arrives with container data
**Then** `onContainers` is called with the parsed `ContainerInfo[]`
**And** a new request is immediately initiated

### Error: Network failure
**Given** the frontend is polling via `useStatePoll`
**When** a request fails (network error or non-200)
**Then** the hook waits with exponential backoff (1s, 2s, 4s, ... up to 30s)
**When** the backoff reaches max retries (10)
**Then** `onError` is called

### Cleanup: Unmount
**Given** the component using `useStatePoll` unmounts
**Then** the in-flight request is aborted
**And** no further requests are made