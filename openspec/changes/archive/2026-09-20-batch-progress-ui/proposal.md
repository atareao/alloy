# Proposal

## Why

La tarjeta de progreso de actualización por lotes (`BatchProgress`) muestra actualmente una lista scrollable con entradas duplicadas y una barra de progreso que no refleja información real. El contador muestra "60/61 zennotes, 61/61 zennotes" porque `batchCurrentItem` se queda con el último nombre de contenedor aunque ya haya terminado. No hay un resumen claro de cuántos contenedores se han revisado, actualizado, o tienen errores.

## What Changes

1. **Eliminar la lista scrollable** de entradas de progreso individuales (no aporta valor, duplica información)
2. **Eliminar contadores en vivo redundantes** (✅ N ok, ❌ N errores, 🔄 N pendientes) que aparecían tanto en checking como en updating
3. **Arreglar el total** para que use siempre `batchProgress.total` (todos los contenedores) en vez de cambiar a `checkResults.updated` durante la fase updating
4. **Derivar el texto del contenedor actual** directamente del `progress` Map (primer entry con `done: false`), mostrando su `status` real (ej: "🔍 Verificando crowdsec:latest...")
5. **Añadir resumen en vivo** debajo de la barra con: total containers, revisados, actualizados, errores, pendientes
6. **Cuando todo termina**, mostrar "✅ Completado" en vez del último nombre de contenedor

## Capabilities

### New Capabilities
- `ui/batch-progress`: Tarjeta de progreso de operaciones por lotes (check/update de contenedores) con barra de progreso, contador, y resumen en vivo de resultados

### Modified Capabilities
- *(ninguno — los specs existentes no cubren BatchProgress)*

## Impact

- **Frontend**: Solo `frontend/src/components/BatchProgress.tsx` — cambios de presentación, sin cambios de lógica de negocio
- **Backend**: Sin cambios
- **Tests**: Los tests existentes de frontend deben seguir pasando sin modificaciones