# Proposal

## Why

El middleware de autenticación acepta el JWT de sesión completo mediante `?token=` en cualquier ruta `/api/*`. Las query strings acaban en logs de acceso, logs de proxies y en la cabecera `Referer`, por lo que un token de sesión de larga duración puede filtrarse. SSE no permite enviar `Authorization` desde el navegador, así que se necesita un credential por query, pero debe ser efímero y de ámbito limitado.

## What Changes

- `GET /api/auth/sse-token` deja de devolver el JWT de sesión y pasa a emitir un **ticket SSE** firmado, con TTL de 60 s y claim `purpose: "sse"`.
- El middleware sólo acepta `?token=` en `GET /api/stream` y sólo si es un ticket SSE válido (`purpose` + `exp`). Rechaza JWT de sesión en query.
- El ticket no se acepta por cookie ni por `Authorization: Bearer`; el JWT de sesión se sigue aceptando por cookie/Bearer.
- La cookie sigue siendo el transporte primario del frontend; no se modifica `useRealtime`.

## Capabilities

### New Capabilities

- `auth/sse-ticket`: emisión y validación de tickets efímeros para SSE.

### Modified Capabilities

- `auth/session`: la validación de sesión rechaza tokens con `purpose`.
- `auth/middleware`: la extracción de credenciales por query se restringe a tickets SSE en `/api/stream`.

## Impact

- `backend/src/models.rs`: claims del ticket y `purpose` opcional en `SessionClaims`.
- `backend/src/auth.rs`: endpoint `sse_token_h`, middleware y tests.
- `docs/architecture.md` y `docs/backend-hardening.md`: documentar el cierre del punto 4.
- Sin cambios de dependencias ni migraciones de datos.
- Issue: #159 punto 4.
