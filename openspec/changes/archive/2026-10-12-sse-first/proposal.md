# Proposal

## Why

El estado en tiempo real de Alloy se sirve hoy mediante **long-polling** (`GET /api/state` + `tokio::sync::Notify`), no por SSE. El SSE solo sobrevive para logs. Sin embargo, el repo ya tiene specs archivadas (`sse`, `frontend/sse`) que **mandatan** un stream SSE único `GET /api/stream`, y el código no las cumple: hay **drift spec↔código**.

El long-poll actual tiene tres problemas de diseño reales:

1. **Avisos perdidos.** `Notify::notify_waiters()` no guarda permiso: si el cambio ocurre entre que el cliente recibe la respuesta y registra el siguiente `notified()`, el aviso se pierde y la latencia sube hasta el timeout de 30 s.
2. **Semántica de snapshot.** Solo se ve el último estado; de ahí los *workarounds* de fin de batch en `App.tsx` para detectar `checking === "__batch__"` y la "race condition" del reset de progreso.
3. **Fan-out manual.** Cada punto de mutación debe acordarse de llamar a `notify_waiters()`.

Además hay infraestructura muerta que confunde: `useSSE.ts` sin usar, `NotifToast.tsx`/`NotifEvent` sin consumidor, `broadcast::Sender<StateEvent>` emitido sin suscriptor, y un comentario obsoleto en `main.rs` que cita endpoints inexistentes.

## What Changes

- Revivir **`GET /api/stream`** como canal SSE único y autoritativo, con eventos tipados:
  - `snapshot` → `StateResponse` (estado completo; se envía **al conectar** para no perder estado tras reconexión).
  - `container-event` → `ContainerEvent` (highlight incremental por contenedor).
  - `update-progress` → `UpdateProgress` (progreso de batch).
  - `notification` → `NotifEvent` (notificaciones de UI).
- Heartbeat por **fusión de streams** cada 5 s (comentario `: keepalive`), según spec `sse` vigente; no `Sse::keep_alive()`.
- Reintroducir los canales `broadcast` necesarios en `AppState` (`progress_tx`, `notif_tx`, `container_event_tx`) y emitir en todos los puntos de mutación.
- Frontend: un único `EventSource` a `/api/stream` con manejadores tipados; resucitar `NotifToast`; highlights en el dashboard; eliminar los *workarounds* de fin de batch.
- **Fallback**: mantener `/api/state` y `useStatePoll` como respaldo si SSE no está disponible o falla repetidamente.
- Reconciliar specs: nuevas reglas en `realtime/stream`; MODIFIED en `frontend/sse` y `sse`; REMOVED de la cláusula de compatibilidad con endpoints que ya no existen.

## Capabilities

### New Capabilities
- `realtime/stream`: contrato del stream SSE único, eventos tipados, highlights, notificaciones, progreso y fallback.

### Modified Capabilities
- `frontend/sse`: nombres/payloads de eventos alineados con `realtime/stream`; se elimina el requisito de compatibilidad con `/api/events|updates|notifications`.
- `sse`: el heartbeat pasa a referirse solo a los endpoints SSE vigentes (`/api/stream` y logs).

## Impact

- Backend: `models.rs` (nuevo `ContainerEvent`), `state.rs` (canales), `events.rs` (handler `/api/stream`), `workers/state.rs` (emisión de highlights), `updates/handlers.rs` (progreso), `notifications.rs`/llamadas (notificaciones UI), `main.rs` (cableado de canales).
- Frontend: `useSSE.ts` (rework a `useRealtime` con fallback), `App.tsx`, `components/DashboardPage.tsx`, `components/NotifToast.tsx`, `types.ts`.
- Sin cambios de Dockerfile ni schema SQLite.

Fuera de alcance: replay con `Last-Event-ID`, paginación de logs, y eliminación del endpoint `sse-token`.
