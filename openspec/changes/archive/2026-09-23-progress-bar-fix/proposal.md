# Fix: Progress bar no se actualiza durante Check/Update manual

## Why

Al realizar una revisión y actualización manual pulsando "Check All", la barra de progreso del `BatchProgress` component se queda en 0% aunque el contador de texto (`XX/NN`) se actualiza correctamente. Esto ocurre porque `batchProgress.current` se inicializa a `0` pero nunca se actualiza con el valor real de `progress.checked` que llega del backend vía state polling.

## What Changes

- **`App.tsx`**: En el callback de `useStatePoll`, después de `setProgress(bp)`, se añade `setBatchProgress({ current: bp.checked, total: bp.total })` para sincronizar el estado local de la barra de progreso con los datos reales del backend.
- **`BatchProgress.test.tsx`**: Nuevo test que verifica que la barra de progreso muestra el porcentaje correcto (0%, 50%, 100%) según `batchProgress.current` / `batchProgress.total`.

## Scope

- **Frontend**: `App.tsx` (state polling callback) y `BatchProgress.test.tsx` (test)
- **No afecta**: backend, API, tests existentes

## Impacto

- La barra de progreso reflejará el progreso real de la operación batch
- UX más coherente: texto y barra se mueven al unísono