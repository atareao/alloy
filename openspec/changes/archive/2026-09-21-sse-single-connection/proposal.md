# Change Proposal: Single SSE Connection

## Why
The browser shows "Stalled" for all SSE connections because three long-lived connections exhaust the browser's connection pool (6 per domain in HTTP/1.1). Reducing to a single connection frees pool slots and allows the browser to establish the connection without queuing.

## What Changes
- **Backend**: New `GET /api/stream` endpoint that merges all three broadcast channels into one SSE stream
- **Frontend**: Replace 3 separate EventSource connections with 1 connection to `/api/stream`
- **Old endpoints**: `/api/events`, `/api/updates`, `/api/notifications` remain functional

## Impact
- Reduces SSE connections from 3 → 1
- Frees 2 connection pool slots
- All event types still delivered in real-time
- No auth changes needed (existing fix stays in place)