# Proposal

## Why

El polling fallback a `GET /api/check-progress` cada 2 segundos durante operaciones batch es redundante:

1. **SSE auto-reconecta**: `EventSource` reconecta automáticamente al caerse. No necesita un fallback por polling.
2. **Broadcast channel tiene buffer**: Tokio usa `broadcast::channel` con capacidad 256. Desconexiones breves no pierden eventos.
3. **State worker cubre el estado final**: El worker de estado (cada 30s) actualiza los contenedores vía SSE `/api/events`. El resultado se refleja igual.
4. **Peticiones HTTP innecesarias**: 1 petición cada 2s durante todo el batch, sin beneficio real.

Pero al eliminar el polling, si alguien recarga la página durante una actualización activa, pierde el tracking visual del progreso. Solución: una **consulta única** a `GET /api/check-progress` al montar la página para recuperar el estado de actualizaciones en curso.

## What Changes

1. **Frontend: Eliminar polling `useEffect`** en `App.tsx` (~35 líneas) que hacía `setInterval` a `/api/check-progress` cada 2s.
2. **Frontend: Añadir recovery `useEffect`** en `App.tsx` (~15 líneas) que llama `GET /api/check-progress` **una sola vez** al montar. Si hay entries activas (`done === false`), setea `batchPhase = "active"` y muestra el `BatchProgress` card.
3. **Tests**: 2 tests nuevos en `App.test.tsx` que verifican la recuperación en recarga.

## Capabilities

### Modified Capabilities
- `ui/batch-progress`: Eliminar polling redundante, añadir recuperación única en recarga de página

## Impact

- **Frontend**: `frontend/src/App.tsx` — eliminar polling useEffect, añadir recovery useEffect
- **Frontend**: `frontend/src/App.test.tsx` — 2 tests nuevos para la recuperación
- **Backend**: Sin cambios (endpoint `GET /api/check-progress` se mantiene para la recuperación)
- **Tests**: 37 tests frontend pasan, 192 tests backend pasan