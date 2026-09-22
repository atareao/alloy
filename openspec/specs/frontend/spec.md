# Spec Delta: Frontend State Endpoint Unificado

## ADDED

### New Types
```typescript
export interface ContainerSummary {
  total: number;
  running: number;
  stopped: number;
  paused: number;
  with_updates: number;
}

export interface StateResponse {
  containers: ContainerInfo[];
  summary: ContainerSummary;
  progress: Record<string, UpdateProgress>;
}
```

## MODIFIED

### `useStatePoll.ts`
- Changed signature from `onContainers: (ContainerInfo[]) => void` to `onState: (StateResponse) => void`
- Parses response as `StateResponse` instead of `ContainerInfo[]`

### `App.tsx`
- State callback now receives `StateResponse` and updates containers + summary + progress
- Progress map updated from `state.progress` instead of polling
- Batch completion detected via `__batch__` entry in `state.progress`
- Removed progress polling `useEffect` (polls `/api/check-progress` every 2s)
- Removed recovery `useEffect` (checks for in-progress updates on page load)
- Removed `clearProgress` callback and `cancelBatchRef`

### `DashboardPage.tsx`
- Added `summary: ContainerSummary` prop
- Removed local computation: `statsRunning`, `statsStopped`, `statsUpdates`
- Uses `summary.running`, `summary.stopped`, `summary.with_updates` directly

## REMOVED

### Removed from App.tsx
- Progress polling `useEffect` (polls `/api/check-progress` every 2s)
- Recovery `useEffect` (checks for in-progress updates on page load)
- `clearProgress` callback
- `cancelBatchRef` ref

## Scenarios

### Happy Path: State + progress in one response
**Given** the frontend calls `useStatePoll`
**When** the response arrives
**Then** `onState` is called with `StateResponse` containing containers, summary, and progress
**Then** containers are updated in state
**Then** progress map is updated from `state.progress`
**Then** a new request is immediately initiated

### Happy Path: Batch complete detected via state
**Given** the frontend receives a `StateResponse`
**When** `state.progress` contains `__batch__` with `done: true`
**Then** batch phase is set to "idle"
**Then** summary dialog is shown
**Then** history and config are refreshed

### Error: Network failure
**Given** the frontend is polling via `useStatePoll`
**When** a request fails
**Then** the hook waits with exponential backoff (1s, 2s, 4s, ... up to 30s)
**When** the backoff reaches max retries (10)
**Then** `onError` is called