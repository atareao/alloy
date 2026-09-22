# frontend/sse Specification

## ADDED Requirements

### Requirement: Single `/api/stream` endpoint merges all event types

The backend MUST provide a `GET /api/stream` endpoint that subscribes to all three broadcast channels (`StateEvent`, `UpdateProgress`, `NotifEvent`) and merges them into a single SSE stream using `futures::stream::select_all`. Each event MUST be dispatched with the correct SSE event name:
- `StateEvent` → `"containers"`
- `UpdateProgress` → `"update-progress"`
- `NotifEvent` → `"notification"`

#### Scenario: Single connection serves all events

- **Given** the user is authenticated
- **When** the frontend creates an EventSource to `/api/stream`
- **Then** the browser establishes a single SSE connection
- **And** receives `containers`, `update-progress`, and `notification` events on that connection

#### Scenario: Connection drops and reconnects

- **Given** the single SSE connection is established
- **When** the connection drops (network error, server restart)
- **Then** the browser auto-reconnects (EventSource built-in behavior)
- **And** resumes receiving all event types

#### Scenario: No events idle

- **Given** the single SSE connection is established
- **When** no events are being produced
- **Then** the connection stays open (keep-alive comments)
- **And** events are received when they are produced

### Requirement: Frontend uses single EventSource

The frontend MUST replace three separate `useEffect` hooks with a single `useEffect` that creates one `EventSource` to `/api/stream` and listens for all three event types (`containers`, `update-progress`, `notification`) on that connection.

#### Scenario: Frontend connects to /api/stream

- **Given** the user is authenticated
- **When** the App component mounts
- **Then** a single EventSource is created to `/api/stream`
- **And** event listeners are registered for `containers`, `update-progress`, and `notification`

### Requirement: Backward compatibility

The old endpoints `/api/events`, `/api/updates`, `/api/notifications` MUST remain functional for backward compatibility.

#### Scenario: Old endpoints still respond

- **Given** the user is authenticated
- **When** a request is made to `/api/events`, `/api/updates`, or `/api/notifications`
- **Then** the endpoint returns `200 OK` with `Content-Type: text/event-stream`