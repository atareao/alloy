# Design: Single Multiplexed SSE Connection

## Problem
Three separate SSE connections (`/api/events`, `/api/updates`, `/api/notifications`) exhaust the browser's connection pool (6 per domain in HTTP/1.1). Each SSE connection is long-lived, consuming a slot permanently. With keep-alive connections from resource downloads, the pool fills up and new SSE requests get queued ("Stalled").

## Solution
Replace 3 SSE connections with 1 multiplexed connection to a new `/api/stream` endpoint.

## Architecture

```
Before:                    After:
┌─ Browser ─┐             ┌─ Browser ─┐
│ /api/events │             │ /api/stream│
│ /api/updates│             └─────┬──────┘
│ /api/notif  │                   │
└─────┬──────┘              ┌─────┴──────┐
      │                     │  Backend   │
┌─────┴──────┐              │ select_all │
│  Backend   │              │ /api/events│
│ 3 handlers │              │ /api/updates│
└────────────┘              │ /api/notif │
                            └────────────┘
```

## Backend changes
- New handler `sse_stream_h` in `events.rs`
- Uses `futures::stream::select_all` to merge 3 broadcast receivers
- Each event dispatched with correct SSE event name
- Route registered in `main.rs`

## Frontend changes
- Single `useEffect` in `App.tsx` connecting to `/api/stream`
- Three event listeners on the same EventSource
- Remove old 3 separate SSE effects

## Why this works
- Reduces SSE connections from 3 → 1
- Frees 2 connection pool slots
- Browser can establish the connection without queuing
- All event types still delivered in real-time