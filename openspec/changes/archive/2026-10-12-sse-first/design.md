# Design

## Context

Hoy el estado llega por long-poll (`events.rs::state_h`, `useStatePoll.ts`). El SSE existe solo para logs (`containers.rs`, `stacks.rs`). Las specs archivadas `sse` y `frontend/sse` describen un `/api/stream` que ya no existe: drift.

Transporte ya resuelto que reutilizamos:
- `auth_middleware` no toca cabeceras en respuestas `text/event-stream` (`auth.rs:450-466`).
- `TCP_NODELAY` por conexión aceptada vía `tap_io` (`main.rs:258-265`).
- Cookie de sesión enviada automáticamente por `EventSource` con `withCredentials`.

## Goals / Non-Goals

**Goals:**
- Un único stream SSE tipado, sin avisos perdidos y con snapshot al conectar.
- Fallback a long-poll si SSE no está disponible.
- Eliminar los *workarounds* de fin de batch.

**Non-Goals:**
- Replay de eventos con `Last-Event-ID` (la reconexión recibe snapshot completo; suficiente).
- Cambiar el modelo de logs (siguen por SSE aparte).
- Retirar el endpoint `sse-token` (se deja, aunque no se use).

## Decisions

### D1: `/api/stream` es la fuente de verdad; el long-poll es respaldo
El handler SSE sirve un único stream; `GET /api/state` se conserva sin cambios para el fallback. Ambos exponen `StateResponse` equivalente.

### D2: Snapshot al conectar
El handler emite primero un evento `snapshot` construido desde `cached_containers` + `compute_summary` + `progress_cache`, y después encadena el stream fusionado. Esto cubre reconexiones sin replay.

### D3: Cuatro canales `broadcast`, un stream fusionado
`AppState` gana `progress_tx: broadcast::Sender<UpdateProgress>`, `notif_tx: broadcast::Sender<NotifEvent>` y `container_event_tx: broadcast::Sender<ContainerEvent>`. El handler usa `futures::stream::select_all` sobre los 4 (`tx` ya existente + los 3 nuevos), fusionado con un `tokio::time::interval(5s)` que emite `: keepalive`.

### D4: Highlights desde `state_worker::refresh`
El worker ya detecta `prev != curr`. Ese mismo punto emite `ContainerEvent { name, action, state, timestamp }`. `action` es la acción Docker normalizada (`start|stop|die|pause|unpause|restart|create|destroy|rename|update`).

### D5: Notificaciones de UI reutilizan el disparador existente
Donde ya se llama a `notify_all(...)` por `notify_events`, se emite además `NotifEvent` por `notif_tx`. No se añaden notificaciones nuevas, solo se exponen las existentes a la UI.

### D6: Progreso por evento, no por snapshot
`update_progress()` y `reset_progress_cache()` emiten `update-progress` además de `notify_waiters()`. El frontend detecta fin de batch por el evento final (`done`/reset), eliminando la heurística `checking === "__batch__"`.

### D7: Fallback gobernado por el cliente
`useRealtime` abre SSE; si `EventSource` no existe o supera un umbral de errores, cambia a `useStatePoll`. Expone `status: "sse" | "polling" | "down"`.

## Risks / Trade-offs

- [Riesgo] Proxies que bufferizan SSE. Mitigación: heartbeat cada 5 s + `TCP_NODELAY` (ya implementado).
- [Riesgo] Duplicidad temporal de caminos (SSE + long-poll) hasta consolidar. Mitigación: el fallback reutiliza el endpoint ya probado.
- [Riesgo] `ContainerEvent` de alta frecuencia en hosts con muchos contenedores. Mitigación: el evento es incremental y ligero; se puede *debounce* en el cliente.
- [Trade-off] Se mantiene `broadcast` con capacidad fija; un cliente lento puede perder eventos intermedios, pero recibe snapshot al reconectar.

## Migration Plan

1. Backend: tipos + canales + emisión + handler `/api/stream`.
2. Frontend: `useRealtime` (SSE con fallback a `useStatePoll`); notificaciones y highlights.
3. Retirar *workarounds* de batch y código muerto (`useSSE`).
4. Rollback: revertir commit; el long-poll sigue operativo en todo momento.

## Resolved Decisions (aprobadas por el usuario)

- **Nombre de capability:** se conserva `frontend/sse` (sin `RENAMED` a `frontend/realtime`).
- **Emisor de `container-event`:** únicamente el worker de estado (`workers/state.rs`). Como observa Docker Events, esto cubre también los cambios producidos por actualizaciones programadas, sin emitir desde los handlers.
