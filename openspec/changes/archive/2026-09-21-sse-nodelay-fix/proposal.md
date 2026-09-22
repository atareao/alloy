# Proposal: sse-nodelay-fix

## Problem

Los eventos SSE (`/api/events`, `/api/updates`, `/api/notifications`, `/api/stream`)
se entregan al navegador "a golpes" (en ráfagas), no instantáneamente al ser emitidos
por el backend.

## Root Cause

En `backend/src/main.rs`, `TCP_NODELAY` se activa sobre el **socket de escucha**
(`socket.set_nodelay(true)` antes de `.bind()`/`.listen()`), pero esta opción de socket
**no se hereda** por las conexiones aceptadas (`accept()`) en Linux. Como resultado,
cada conexión de cliente real usa el comportamiento por defecto (Nagle's algorithm
habilitado), que agrupa escrituras pequeñas (como los chunks SSE `event: ...\ndata: ...\n\n`)
antes de enviarlas por la red, introduciendo latencia y "ráfagas" perceptibles.

`axum::serve` (0.8) expone `axum::serve::ListenerExt::tap_io` precisamente para resolver
este caso: permite ejecutar una clausura sobre cada `TcpStream` aceptado, donde se puede
aplicar `set_nodelay(true)` por conexión.

## Scope

- Modificar el arranque del servidor en `backend/src/main.rs` para envolver el listener
  con `.tap_io(...)` y aplicar `TCP_NODELAY` a cada conexión aceptada.
- Eliminar la llamada incorrecta `socket.set_nodelay(true)` sobre el socket de escucha
  (no tiene efecto útil y puede eliminarse o mantenerse sin impacto — se documentará
  la decisión).
- No se modifica la lógica de generación/emisión de eventos SSE en `events.rs` (ya hace
  flush inmediato vía `Body::from_stream`).

## Out of Scope

- Buffering en un reverse-proxy externo (nginx/Traefik) — fuera del control del backend,
  ya se envía `X-Accel-Buffering: no` en `/api/stream`.
- Cambios en el cliente (`useSSE.ts`, `EventSource` nativo) — no hay evidencia de
  buffering en el lado del navegador.

## Impact

- Módulo afectado: `backend/src/main.rs` (arranque del servidor).
- Sin cambios de API pública ni de contratos de datos.
- Reduce la latencia de entrega de eventos SSE a los clientes conectados.
