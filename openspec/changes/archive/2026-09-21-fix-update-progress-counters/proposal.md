# Proposal

## Why

El progreso de actualización por lotes muestra valores incorrectos en el frontend:
1. **Contadores a cero en `apply_single_policy`**: Cada llamada a `update_progress` dentro de `apply_single_policy` pasa `total=0, checked=0, updated=0, errors=0`, contaminando la caché de progreso y el mapa de progreso del frontend con entradas de valor cero.
2. **Sin logs en frontend**: El manejador SSE de `update-progress` no tiene `console.log`, lo que imposibilita depurar desde el navegador.
3. **Bug `0 ?? current` en `BatchProgress.tsx`**: `latestProgress?.checked ?? current` devuelve `0` cuando `checked` es `0` (porque `0` no es null/undefined), mostrando "0 revisados" en el resumen aunque el backend haya revisado decenas de contenedores.
4. **Salto a "3/61"**: Si el usuario hace clic en "Check All" mientras ya hay un lote en ejecución, `batchProgress` se reinicia a `{ current: 0, total: containers.length }` y los eventos SSE del nuevo lote empiezan desde `checked=1`.

## What Changes

1. **Backend: Pasar contadores reales a `apply_single_policy`**: Añadir parámetros `total`, `checked`, `updated`, `errors` a `apply_single_policy` y pasarlos desde `check_and_apply_all` a todas las llamadas `update_progress` dentro de `apply_single_policy`.
2. **Frontend: Añadir `console.log` en manejador SSE**: Para poder depurar desde la consola del navegador.
3. **Frontend: Arreglar `0 ?? current` en `BatchProgress.tsx`**: Usar `??` correctamente o un fallback explícito para cuando `checked` es `0`.
4. **Frontend: Prevenir reinicio de `batchProgress` durante lote activo**: No reiniciar `batchProgress` en `checkAll` si ya hay un lote activo.

## Capabilities

### Modified Capabilities
- `updates/progress`: Flujo de progreso de actualización por lotes — backend envía contadores correctos desde `apply_single_policy`
- `ui/batch-progress`: Tarjeta de progreso — arreglar visualización de contadores cuando `checked=0`

## Impact

- **Backend**: `backend/src/updates/handlers.rs` — cambiar firma de `apply_single_policy` y todas las llamadas a `update_progress` dentro de ella
- **Frontend**: `frontend/src/App.tsx` — añadir `console.log` en SSE handler
- **Frontend**: `frontend/src/components/BatchProgress.tsx` — arreglar `0 ?? current`
- **Tests**: Los tests existentes deben seguir pasando sin modificaciones