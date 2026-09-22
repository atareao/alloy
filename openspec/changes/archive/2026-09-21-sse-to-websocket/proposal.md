# Proposal: sse-to-websocket

## Why

Los eventos SSE llegan al navegador en ráfagas debido al buffering de HTTP/2
en Traefik. Se han intentado sin éxito:
1. `TCP_NODELAY` por conexión aceptada (`sse-nodelay-fix`)
2. Exclusión de `text/event-stream` de la compresión gzip en Traefik
3. Heartbeat periódico cada 5s para forzar frames HTTP/2 (`sse-heartbeat`)

La causa raíz es que SSE viaja sobre HTTP, y HTTP/2 en Traefik bufferiza los
frames de datos. Los WebSockets (`ws://`/`wss://`) no tienen este problema
porque:
- La conexión empieza con un upgrade HTTP/1.1 (no HTTP/2)
- Tras el upgrade, los mensajes viajan sobre TCP directo con su propio
  framing, sin pasar por el pipeline HTTP/2 de Traefik
- Traefik maneja WebSocket upgrades de forma transparente sin buffering

## What Changes

### Backend

1. **Nuevo endpoint** `GET /api/ws` que realiza upgrade WebSocket usando
   `axum::extract::ws::WebSocketUpgrade`. Autenticación vía `?token=` en
   query string (ya soportado por `auth_middleware`).
2. El handler de WebSocket recibe eventos de los 3 broadcast channels
   (`StateEvent`, `UpdateProgress`, `NotifEvent`) y los envía como mensajes
   JSON con un campo `type` para distinguirlos:
   ```json
   {"type": "containers", "data": {...}}
   {"type": "update-progress", "data": {...}}
   {"type": "notification", "data": {...}}
   ```
3. Se añade `axum = { version = "0.8", features = ["ws"] }` a Cargo.toml
   (la feature `ws` de axum para WebSocket support).
4. Los endpoints SSE existentes (`/api/events`, `/api/updates`,
   `/api/notifications`, `/api/stream`) se mantienen operativos pero quedan
   deprecados (no se eliminan para no romper clientes existentes).

### Frontend

1. Nuevo hook `useWS.ts` que reemplaza `useSSE.ts`:
   - Conecta vía `new WebSocket("wss://.../api/ws?token=...")`
   - Obtiene el token de sesión vía `GET /api/auth/sse-token` (endpoint ya
     existente del proposal `sse-direct-bypass` que se descartó pero el
     endpoint es útil aquí)
   - Reintentos con backoff exponencial (misma lógica que `useSSE.ts`)
   - Filtra mensajes por `type` y llama al callback correspondiente
2. `App.tsx` y `DashboardPage.tsx` se actualizan para usar `useWS` en lugar
   de `new EventSource(...)` directo.
3. `useSSE.ts` se mantiene pero queda deprecado.

### Traefik

Sin cambios necesarios — Traefik maneja WebSocket upgrades de forma
transparente.

## Impact

- Módulos backend: `backend/src/events.rs` (nuevo handler WS),
  `backend/Cargo.toml` (feature `ws`), `backend/src/main.rs` (registrar ruta)
- Módulos frontend: `frontend/src/useWS.ts` (nuevo), `frontend/src/App.tsx`,
  `frontend/src/components/DashboardPage.tsx`
- Los endpoints SSE actuales siguen funcionando (retrocompatibilidad)
- Sin cambios en Traefik ni infraestructura

## Out of Scope

- Eliminar los endpoints SSE legacy (se mantienen por ahora)
- WebSocket bidireccional (solo server→client, como SSE)
- Otros clientes que no sean el frontend web