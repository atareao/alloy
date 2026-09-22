# Proposal: sse-heartbeat

## Why

Los eventos SSE llegan al navegador en ráfagas a pesar de:
1. Haber corregido `TCP_NODELAY` por conexión aceptada (`sse-nodelay-fix`)
2. Haber excluido `text/event-stream` de la compresión gzip en Traefik
3. Confirmar que no hay CDN ni otro proxy intermedio

El tráfico entre navegador y Traefik usa **HTTP/2**. En HTTP/2, los datos se
transmiten en *frames* (tamaño máximo por defecto: 16 KB). Cuando el backend
envía eventos SSE pequeños (unos pocos cientos de bytes cada uno), Traefik
puede acumularlos hasta llenar un frame HTTP/2 antes de transmitirlos al
cliente, produciendo el efecto de ráfaga.

La solución estándar y probada para SSE sobre HTTP/2/proxies es añadir un
**heartbeat periódico** (comentario SSE `: keepalive\n\n`) que fuerza el
envío de un frame HTTP/2 aunque no haya datos "reales", rompiendo la
acumulación y haciendo que los eventos se entreguen inmediatamente al
cliente.

## What Changes

Añadir un heartbeat periódico (cada 5 segundos) a los 4 endpoints SSE:

1. **`/api/events`** (`sse_events_h` en `events.rs`)
2. **`/api/updates`** (`sse_updates_h` en `events.rs`)
3. **`/api/notifications`** (`sse_notifications_h` en `events.rs`)
4. **`/api/stream`** (`sse_stream_h` en `events.rs`)

El heartbeat se implementa fusionando el stream de eventos reales con un
stream de intervalo (`tokio::time::interval`) que emite el comentario
`: keepalive\n\n`. Se usa `futures::stream::select` o `stream::select_all`
para intercalar ambos streams.

## Impact

- Módulo afectado: solo `backend/src/events.rs`
- Sin cambios de API, sin cambios de contrato, sin cambios en el frontend
- El heartbeat es invisible para el cliente (`EventSource` ignora líneas
  que empiezan con `:` por definición del protocolo SSE)
- Sin impacto en rendimiento (5s es un intervalo conservador)

## Out of Scope

- Cambios en Traefik (fuera del repo)
- Cambios en el frontend
- Cambios en la lógica de negocio de los eventos