# Proposal

## Why

El bump de `oxlint` 1.74 → 1.94 (#163 fase 1) activa por defecto la regla `react(set-state-in-effect)`, que hoy emite **5 warnings** en el frontend. No bloquean CI (exit 0), pero impiden añadir `--deny-warnings` al script `lint` y dejan deuda visible.

Los warnings reales en `develop` son:

- `frontend/src/components/TelegramCard.tsx:42`
- `frontend/src/components/ValetCard.tsx:41`
- `frontend/src/components/MatrixCard.tsx:42`
- `frontend/src/components/DashboardPage.tsx:222`
- `frontend/src/App.tsx:387`

## What Changes

- **Tres `*Card` (Telegram/Valet/Matrix)**: sustituyen el `useEffect` que sincroniza el estado del formulario desde `configProp` por el patrón "ajustar estado durante el render" (comparar la prop con la anterior y aplicar los setters en el render), extraído a un hook compartido `frontend/src/useConfigSync.ts`. Se elimina el `useEffect` y la duplicación de la lógica de sincronización.
- **`DashboardPage` logs**: los reseteos síncronos (`setLogs([])`, `setLogError(null)`, `setLogTimeout(false)`) se mueven al event handler `handleLogs`, que es quien provoca el cambio. El `useEffect` se queda solo con el `EventSource` y el timer (sistema externo).
- **`App.tsx`**: `fetchCheckConfig` es una función async cuyo `setState` ocurre **después de `await`**, por lo que es un falso positivo de la regla. Se añade un `oxlint-disable-next-line react/set-state-in-effect` con comentario justificativo, en vez de reestructurar el efecto.
- **Script `lint`**: `oxlint` → `oxlint --deny-warnings`, para que los warnings rompan el comando.
- **Tests**: se añaden/ajustan tests que fijan el comportamiento preservado (precarga de campos desde `configProp`, token limpio tras guardar, switch activado por configuración parcial, `EventSource` de logs con la URL correcta) y que el lint no emite warnings.

## Capabilities

### New Capabilities

- `frontend/lint`: política de lint sin warnings y patrón de sincronización de estado desde props sin `useEffect`.

### Modified Capabilities

Ninguna.

## Impact

- `frontend/src/components/TelegramCard.tsx`, `ValetCard.tsx`, `MatrixCard.tsx`, `DashboardPage.tsx`, `App.tsx`.
- Nuevo `frontend/src/useConfigSync.ts`.
- `frontend/package.json` (script `lint`).
- Tests: `frontend/src/components/ConfigPage.test.tsx`, `frontend/src/components/DashboardPage.test.tsx`.
- Issue: #179.
