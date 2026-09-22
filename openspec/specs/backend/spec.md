# Spec Delta: Backend State Endpoint

## ADDED

### New Route
```
GET /api/state  → 200 JSON: Vec<ContainerInfo>
```

### New Handler: `state_h`
```rust
async fn state_h(
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(cached): State<CachedContainers>,
) -> Json<Vec<ContainerInfo>>
```

**Behavior**:
1. Subscribe to `state_tx` broadcast channel
2. Wait up to 30 seconds for the next `StateEvent` using `tokio::time::timeout`
3. If event arrives before timeout, return `Json(evt.containers)` with the updated container list
4. If timeout expires (or broadcast error), read from `cached_containers` and return `Json(cached.clone().unwrap_or_default())`
5. Response headers: `Cache-Control: no-cache`

## REMOVED

### Removed Routes
- `GET /api/events` — SSE container events
- `GET /api/updates` — SSE update progress
- `GET /api/notifications` — SSE notifications
- `GET /api/stream` — Multiplexed SSE
- `GET /api/ws` — WebSocket

### Removed Handlers
- `sse_events_h`, `sse_updates_h`, `sse_notifications_h`, `sse_stream_h`
- `ws_h`, `handle_ws`

### Removed Broadcast Channels
- `update_tx` (UpdateProgress) — no longer needed for real-time delivery
- `notif_tx` (NotifEvent) — no longer needed for real-time delivery
- Keep `tx` (StateEvent) — still used by state worker and state endpoint

## MODIFIED

### `events.rs`
- Replaced all SSE/WebSocket handlers with single `state_h` handler
- Updated `routes()` to return only `GET /api/state`
- Updated tests: removed SSE/WS tests, added state endpoint tests

### `main.rs`
- Removed `update_tx` and `notif_tx` broadcast channel creation
- Removed references to `update_tx`/`notif_tx` in `AppState` construction
- Removed `notif_tx` parameter from `state_worker` spawn
- Removed `update_tx` and `notif_tx` parameters from `update_check_worker` spawn

### `state.rs`
- Removed `update_tx` and `notif_tx` fields from `AppState`
- Removed `FromRef` impls for `broadcast::Sender<UpdateProgress>` and `broadcast::Sender<NotifEvent>`

### `workers/state.rs`
- Removed `notif_tx` parameter from `refresh()` and `state_worker()`
- Removed `notif_tx.send(NotifEvent { ... })` call

### `workers/scheduler.rs`
- Removed `update_tx` and `notif_tx` parameters from `update_check_worker()`
- Removed all `update_tx.send(...)` and `notif_tx.send(...)` calls

### `stacks.rs`
- Removed `update_tx` and `notif_tx` State extracts from `update_stack_h()`
- Removed all `update_tx.send(...)` and `notif_tx.send(...)` calls

### `updates/handlers.rs`
- Removed `update_tx` parameter from `update_progress()`
- Removed `update_tx`/`notif_tx` State extracts from handlers
- Removed `update_tx`/`notif_tx` parameters from helper functions
- Removed all `update_tx.send(...)` and `notif_tx.send(...)` calls

## Scenarios

### Happy Path: State changes within timeout
**Given** a client calls `GET /api/state`
**When** a container state change occurs within 30 seconds
**Then** the response returns HTTP 200 with the updated `Vec<ContainerInfo>`

### Happy Path: No state changes (timeout)
**Given** a client calls `GET /api/state`
**When** no container state change occurs within 30 seconds
**Then** the response returns HTTP 200 with the current `Vec<ContainerInfo>` from cache

### Happy Path: First call (cache empty)
**Given** a client calls `GET /api/state`
**When** `cached_containers` is `None` (not yet populated)
**Then** the handler subscribes and waits for the first `StateEvent`
**When** the state worker sends the first event
**Then** the response returns HTTP 200 with the container list

### Error: Broadcast channel closed
**Given** a client calls `GET /api/state`
**When** the `state_tx` broadcast channel is closed
**Then** the handler returns HTTP 200 with the current cached containers (if any) or an empty array

### Frontend: Continuous polling
**Given** the frontend receives a response from `GET /api/state`
**Then** it immediately makes another request to `GET /api/state`
**When** the request fails (network error, non-200 status)
**Then** it retries with exponential backoff (1s, 2s, 4s, ... up to 30s max)