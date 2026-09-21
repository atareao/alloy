# auth/session Specification

## Purpose
TBD - created by archiving change session-expiry-fix. Update Purpose after archive.

## Requirements

### Requirement: auth_me enforces session idle timeout
The `/api/auth/me` endpoint MUST reject sessions whose idle time exceeds `session_idle_timeout_minutes` by returning HTTP 401 with `session_expired: true`.

#### Scenario: Active session returns authenticated
- **Given** a valid session cookie with `last_active` within idle timeout and `iat` within max duration
- **When** `GET /api/auth/me` is called
- **Then** the response is `200 OK` with `authenticated: true`
- **And** the response includes `session.idle_remaining_secs > 0`
- **And** the response includes `session.session_remaining_secs > 0`

#### Scenario: Session rejected after idle timeout
- **Given** a valid session cookie with `last_active` set to 31 minutes ago
- **And** `session_idle_timeout_minutes` is 30
- **When** `GET /api/auth/me` is called
- **Then** the response is `401 Unauthorized`
- **And** `body.session_expired` is `true`
- **And** `body.detail` contains "inactivity timeout"

### Requirement: auth_me enforces session max duration
The `/api/auth/me` endpoint MUST reject sessions whose total lifetime exceeds `session_max_duration_hours` by returning HTTP 401 with `session_expired: true`.

#### Scenario: Session rejected after max duration
- **Given** a valid session cookie with `iat` set to 25 hours ago
- **And** `session_max_duration_hours` is 24
- **When** `GET /api/auth/me` is called
- **Then** the response is `401 Unauthorized`
- **And** `body.session_expired` is `true`
- **And** `body.detail` contains "maximum duration"

### Requirement: auth_me handles invalid or missing tokens gracefully
The `/api/auth/me` endpoint MUST return `200 OK` with `authenticated: false` (not a 401) for invalid, malformed, or absent session tokens.

#### Scenario: Invalid token returns unauthenticated
- **Given** a malformed or tampered session cookie
- **When** `GET /api/auth/me` is called
- **Then** the response is `200 OK` with `authenticated: false`
- **And** `session_expired` is not present

#### Scenario: No cookie returns unauthenticated
- **Given** no session cookie
- **When** `GET /api/auth/me` is called
- **Then** the response is `200 OK` with `authenticated: false`
