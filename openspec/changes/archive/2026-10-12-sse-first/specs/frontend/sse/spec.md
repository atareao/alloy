# Spec Delta

## MODIFIED Requirements

### Requirement: Single `/api/stream` endpoint merges all event types

The backend MUST provide a `GET /api/stream` endpoint that subscribes to all broadcast channels (`StateEvent`, `UpdateProgress`, `NotifEvent` and the new `ContainerEvent`) and merges them into a single SSE stream using `futures::stream::select_all`. Each event MUST be dispatched with the correct SSE event name:
- full state snapshot → `"snapshot"` (payload `StateResponse`)
- incremental container state change → `"container-event"` (payload `ContainerEvent`)
- batch update progress → `"update-progress"` (payload `UpdateProgress`)
- UI notification → `"notification"` (payload `NotifEvent`)

#### Scenario: Single connection serves all events

- **Given** the user is authenticated
- **When** the frontend creates an EventSource to `/api/stream`
- **Then** the browser establishes a single SSE connection
- **And** receives `snapshot`, `container-event`, `update-progress` and `notification` events on that connection

#### Scenario: Connection drops and reconnects

- **Given** the single SSE connection is established
- **When** the connection drops (network error, server restart)
- **Then** the browser auto-reconnects (EventSource built-in behavior)
- **And** the first event after reconnect is a full `snapshot`

#### Scenario: No events idle

- **Given** the single SSE connection is established
- **When** no events are being produced
- **Then** the connection stays open (keep-alive comments)
- **And** events are received when they are produced

## REMOVED Requirements

### Requirement: Backward compatibility

**Reason**: The endpoints `/api/events`, `/api/updates` and `/api/notifications` do not exist in the current codebase; the state is served by `/api/stream` with the long-poll `/api/state` as fallback. Keeping this requirement mandates endpoints that no longer exist.
**Migration**: Clients use `GET /api/stream` (primary) or `GET /api/state` (fallback).
