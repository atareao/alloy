# auth/middleware Specification

## Purpose
TBD - created by archiving change sse-header-fix. Update Purpose after archive.

## Requirements

### Requirement: auth_middleware skips header modification for SSE responses

The `auth_middleware` function MUST detect SSE responses (Content-Type: `text/event-stream`) and skip all header modifications (including sliding session Set-Cookie) to avoid buffering the SSE stream.

#### Scenario: SSE response does not get Set-Cookie

- **Given** a valid session with `last_active` past the sliding threshold
- **And** a request to `/api/updates` (which returns `Content-Type: text/event-stream`)
- **When** the `auth_middleware` processes the response
- **Then** the response has `Content-Type: text/event-stream`
- **And** the response does NOT have a `Set-Cookie` header
- **And** the response status is `200 OK`

#### Scenario: Non-SSE response does get Set-Cookie

- **Given** a valid session with `last_active` past the sliding threshold
- **And** a request to `/api/containers` (which returns `Content-Type: application/json`)
- **When** the `auth_middleware` processes the response
- **Then** the response has `Content-Type: application/json`
- **And** the response DOES have a `Set-Cookie` header
- **And** the response status is `200 OK`
