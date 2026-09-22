# Proposal

## Why

The SSE endpoints (`/api/events`, `/api/updates`, `/api/notifications`) no llegan al frontend. El backend logea que los eventos se envían al `broadcast::channel` y el SSE handler los recibe y reenvía, pero el navegador no recibe ningún dato (Preview/Response vacío, "Stalled" en Timing). La causa es que el `auth_middleware` modifica los headers de la respuesta (sliding session → `Set-Cookie`) después de que el `Sse` response se ha creado, lo que provoca que Axum bufferée toda la respuesta SSE en lugar de streamearla.

## What Changes

- **`auth.rs` — `auth_middleware`**: Detectar si la respuesta es SSE (`Content-Type: text/event-stream`) y saltar la modificación de headers (sliding session) en ese caso. Las rutas SSE siguen protegidas por auth (el middleware sigue verificando la cookie de sesión), pero no se aplica el sliding session que interfiere con el streaming.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `auth/middleware`: El middleware de autenticación debe detectar respuestas SSE y no modificar sus headers para no interferir con el streaming.

## Impact

- **Archivo modificado**: `backend/src/auth.rs` — función `auth_middleware`
- **No hay cambios de API**: las rutas SSE siguen protegidas, solo cambia el comportamiento interno del middleware
- **No hay cambios de dependencias**