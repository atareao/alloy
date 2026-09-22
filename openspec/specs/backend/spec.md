# Spec Delta: Backend State Endpoint

## ADDED

### New Structs

```rust
#[derive(Clone, Debug, Serialize)]
pub struct ContainerSummary {
    pub total: usize,
    pub running: usize,
    pub stopped: usize,
    pub paused: usize,
    pub with_updates: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateResponse {
    pub containers: Vec<ContainerInfo>,
    pub summary: ContainerSummary,
    pub progress: std::collections::HashMap<String, UpdateProgress>,
}

pub fn compute_summary(containers: &[ContainerInfo]) -> ContainerSummary
```

### New AppState Field
```rust
pub state_notify: Arc<tokio::sync::Notify>,
```

### New Route
```
GET /api/state  → 200 JSON: StateResponse
```

### New Handler: `state_h`
```rust
async fn state_h(
    State(cached_containers): State<CachedContainers>,
    State(progress_cache): State<Arc<Mutex<HashMap<String, UpdateProgress>>>>,
    State(state_notify): State<Arc<tokio::sync::Notify>>,
) -> impl IntoResponse
```

**Behavior**:
1. Wait on `state_notify.notified()` with `tokio::time::timeout(Duration::from_secs(30))`
2. When notified or timeout expires, read cache and progress
3. Compute `ContainerSummary` from containers
4. Return `Json(StateResponse { containers, summary, progress })` with `Cache-Control: no-cache`

### Notify Calls
- `workers/state.rs` → `refresh()` calls `state_notify.notify_waiters()` after cache update
- `updates/handlers.rs` → `update_progress()` calls `state_notify.notify_waiters()` after cache insert

## REMOVED

### Removed Route
- `GET /api/check-progress` — no longer needed

### Removed File
- `backend/src/progress.rs` — entire file removed

## MODIFIED

### `events.rs`
- Replaced broadcast subscriber pattern with `state_notify` wait
- Handler now returns `StateResponse` instead of `Vec<ContainerInfo>`
- Updated tests

### `workers/state.rs`
- Added `state_notify` parameter to `refresh()` and `state_worker()`
- Calls `state_notify.notify_waiters()` after cache update

### `updates/handlers.rs`
- Added `state_notify` parameter to `update_progress()`
- Calls `state_notify.notify_waiters()` after cache insert

### `main.rs`
- Added `state_notify: Arc::new(tokio::sync::Notify::new())` to AppState construction
- Passed `state_notify` to state_worker spawn
- Removed `progress::routes()` from router
- Removed `mod progress;`

### `state.rs`
- Added `state_notify` field to `AppState`
- Added `FromRef` impl for `Arc<tokio::sync::Notify>`

## Scenarios

### Happy Path: Container stops, frontend reflects immediately
**Given** a client is long-polling `GET /api/state`
**When** a container stops (Docker event)
**Then** the state worker updates cache and calls `state_notify.notify_waiters()`
**Then** the waiting handler wakes up, reads updated cache, returns `StateResponse`
**Then** the frontend receives the response and updates the UI

### Happy Path: Batch update progress
**Given** a client is long-polling `GET /api/state`
**When** `update_progress()` is called during a batch operation
**Then** it updates progress_cache and calls `state_notify.notify_waiters()`
**Then** the waiting handler wakes up, returns `StateResponse` with updated progress

### Happy Path: No changes (timeout)
**Given** a client is long-polling `GET /api/state`
**When** no state or progress changes occur within 30 seconds
**Then** the handler returns the current cache after timeout
**Then** the frontend receives the response and immediately re-polls

### Happy Path: Summary computation
**Given** a list of containers with various states
**When** `compute_summary()` is called
**Then** it returns correct counts: total, running, stopped, paused, with_updates

### Error: Cache empty on first call
**Given** a client calls `GET /api/state`
**When** `cached_containers` is `None` (not yet populated)
**Then** the handler waits for the first notify from state worker
**When** the state worker sends its first refresh
**Then** the handler returns the populated cache