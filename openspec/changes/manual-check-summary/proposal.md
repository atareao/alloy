# Proposal

## Why

Tras un check manual, el `SummaryDialog` muestra siempre **0** en Comprobados, Actualizables, Actuales y Errores, aunque el batch sí haya comprobado/actualizado contenedores. Dos causas: (a) el evento de fin de batch se emite con todos los contadores a cero (`reset_progress_cache`), y (b) el frontend (`finishBatch`) no escribe esos contadores en el estado que lee el diálogo, y `BatchProgress` ni siquiera modela *actualizables* ni *actuales*.

## What Changes

- **Backend**: `check_and_apply_all` contabiliza además `updatable` (con actualización disponible), `uptodate` (al día) y `failed` (fallos al aplicar), y el evento de fin de batch (`done`) transporta los contadores **reales** —no un reset a cero—.
- **Backend**: `reset_progress_cache` deja de emitir `done=true` con ceros; limpia la caché y emite un evento *idle* sin señal de fin, para no pisar el resumen real.
- **API**: `UpdateProgress`/`BatchProgress` amplían campos (`updatable`, `uptodate`, `failed`). `/api/check-all` mantiene su retorno (`Vec<ContainerInfo>`).
- **Frontend**: `finishBatch` rellena `checkResults` (Comprobados/Actualizables/Actuales/Errores) y `updateResults` (Actualizados/Fallos) desde los contadores del evento de fin; `SummaryDialog` muestra datos reales.
- **Tests**: backend (contadores y evento de fin) y frontend (`finishBatch` → estados; render del `SummaryDialog`).

## Capabilities

### New Capabilities

- `ui/check-summary`: contrato del resumen post-batch (contadores reales de comprobación y actualización).

### Modified Capabilities

- `updates/update-check`: el fin de batch expone contadores reales (comprobados/actualizables/actuales/actualizados/fallos/errores) en lugar de ceros.

## Impact

- **Backend**: `updates/handlers.rs` (`check_and_apply_all`, `update_progress`, `reset_progress_cache`), `models.rs` (`UpdateProgress`, `BatchProgress`).
- **Frontend**: `App.tsx` (`finishBatch`, `checkAll`), `components/SummaryDialog.tsx`, `types.ts`.
- **API/SSE**: evento `update-progress` con campos nuevos y fin de batch con contadores reales.
- **Dependencias**: ninguna.
