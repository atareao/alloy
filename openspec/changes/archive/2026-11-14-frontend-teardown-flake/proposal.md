# Proposal

## Why

El job `frontend` de CI falla de forma intermitente con
`ReferenceError: window is not defined` (77 tests en verde, pero el proceso sale con exit 1),
lo que bloquea PRs sin motivo real. La causa está verificada: `src/App.tsx` usa la API
**estática** de antd (`notification.success`), que monta un root de React propio en
`document.body` fuera del contenedor de Testing Library. `cleanup()` no lo desmonta, así que el
árbol huérfano sobrevive al test con sus timers vivos y un *passive effects flush* agendado como
macrotask; si el teardown de jsdom gana la carrera, ese callback lee `window.event` y revienta.

## What Changes

- `afterEach` global en `src/test-setup.ts` que, después de cada test:
  1. cierra/desmonta los holders de la API estática de antd (`notification.destroy()` **y**
     `message.destroy()`),
  2. desmonta los árboles de Testing Library (`cleanup()`),
  3. drena el trabajo pendiente del scheduler de React (flush de macrotareas) mediante un helper
     exportado `flushReactWork()`.
- Nuevo `src/test-setup.test.tsx` con la regresión determinista:
  - dos tests encadenados donde el segundo comprueba que **no** queda ningún `.ant-notification`
    montado al empezar (hoy falla),
  - test del mecanismo de `flushReactWork()` (debe drenar macrotareas, no sólo microtareas),
  - test de idempotencia/seguridad con la suite sin nada montado.
- `docs/frontend-test-isolation.md` describiendo el invariante, el harness de reproducción y las
  limitaciones.

No hay cambios **BREAKING** y no se toca la API estática de antd en producción (el `App` real no
se desmonta, así que el holder persistente es correcto allí).

## Capabilities

### New Capabilities

- `frontend/test-isolation`: contrato de aislamiento de la suite de tests del frontend — cada test
  MUST terminar sin dejar holders de la API estática de antd montados ni trabajo del scheduler de
  React pendiente, y el teardown del entorno MUST NOT producir errores no gestionados.

### Modified Capabilities

- Ninguna. `ci/frontend-verification` no cambia sus requisitos (sigue ejecutando `pnpm run test`;
  lo que cambia es que la suite deja de ser flaky por sí misma).

## Impact

- `frontend/src/test-setup.ts` (afterEach global + helper `flushReactWork`).
- `frontend/src/test-setup.test.tsx` (nuevo).
- `docs/frontend-test-isolation.md` (nuevo).
- Sin cambios en código de producción (`src/App.tsx` y el resto de componentes quedan igual) ni en
  `.github/workflows/ci.yml`.
