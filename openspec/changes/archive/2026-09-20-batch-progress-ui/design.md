# Design

## Context

Ver proposal.md — Why. El componente `BatchProgress.tsx` actualmente renderiza una lista scrollable de entradas individuales, contadores redundantes, y un contador que se queda con el último nombre de contenedor. Es un componente de presentación pura: recibe props y renderiza. No hay lógica de negocio, solo transformación de datos del Map a texto.

## Goals / Non-Goals

**Goals:**
- Eliminar la lista scrollable de entradas individuales
- Eliminar contadores redundantes (✅ N ok, ❌ N errores, 🔄 N pendientes) que aparecen duplicados en checking y updating
- Mostrar el contenedor actual con su acción real derivada del progress Map
- Mostrar resumen en vivo con total, revisados, actualizados, errores, pendientes
- Usar siempre `batchProgress.total` como total

**Non-Goals:**
- No cambiar la lógica de negocio en App.tsx (monitoring effect, checkAll, etc.)
- No cambiar el backend
- No añadir nuevas dependencias

## Decisions

- **Derivar currentEntry del Map en vez de usar batchCurrentItem**: El `batchCurrentItem` solo tiene el nombre del contenedor, no su estado. Derivar del Map permite obtener el `status` completo (ej: "🔍 Verificando crowdsec:latest..."). Además evita el bug de que se quede con el último nombre al terminar.
- **Calcular contadores del resumen desde el Map**: Los contadores `liveChecked`, `liveUpdated`, `liveErrors`, `livePending` se calculan con `Array.from(progress.values()).filter(...)` en cada render. Es eficiente porque el Map rara vez supera 100 entradas.
- **No tocar App.tsx**: El monitoring effect ya cuenta correctamente `doneCount` para todos los containers (incluyendo fallos). El problema visual se resuelve enteramente en BatchProgress.tsx.

## Risks / Trade-offs

- [Riesgo] Los tests existentes de frontend podrían fallar si dependen de la lista scrollable → Mitigación: los tests actuales (18 tests) no testean BatchProgress directamente, solo App.test.tsx, LoginScreen.test.tsx, etc.
- [Riesgo] `batchCurrentItem` y `checkResults` siguen en la interfaz pero no se usan en el JSX → Mitigación: se mantienen en la interfaz para compatibilidad, el compilador TypeScript no se queja.