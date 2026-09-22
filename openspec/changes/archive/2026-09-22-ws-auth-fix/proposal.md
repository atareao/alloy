# Proposal: ws-auth-fix

## Why

El endpoint WebSocket `/api/ws` falla con "WebSocket connection failed" porque
el middleware de autenticación (`auth_middleware` en `auth.rs`) interfiere con
la respuesta de upgrade WebSocket.

Cuando el handler WebSocket devuelve `101 Switching Protocols`, el middleware:
1. Detecta que NO es SSE (no tiene `Content-Type: text/event-stream`)
2. No retorna early
3. Ejecuta el sliding session refresh, que intenta añadir un header
   `Set-Cookie` a la respuesta 101

Modificar los headers de una respuesta 101 Switching Protocols corrompe el
upgrade WebSocket, causando que el navegador rechace la conexión.

## What Changes

En `auth_middleware` (`backend/src/auth.rs`), después del bloque SSE check
(línea 463-466), añadir un check adicional: si la respuesta tiene status
`101 Switching Protocols`, retornar early sin modificar headers.

```rust
// WebSocket upgrade — no modificar headers (101 Switching Protocols)
if response.status() == StatusCode::SWITCHING_PROTOCOLS {
    return Ok(response);
}
```

## Impact

- Solo afecta a `backend/src/auth.rs`
- No cambia el comportamiento para SSE ni para API REST normal
- Es un fix de 3 líneas

## Out of Scope

- Cambios en el handler WebSocket (funciona correctamente)
- Cambios en el frontend