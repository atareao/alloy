# Warnings `react(set-state-in-effect)` eliminados

Change: `lint-set-state-in-effect` · Issue #179 · Plan `plans/PLAN-024.md`

## Qué es

El bump de `oxlint` 1.74 → 1.94 (#163 fase 1) activó por defecto la regla `react(set-state-in-effect)`, que emitía **5 warnings** en el frontend. Este change los elimina y activa `--deny-warnings` en el script `lint`, de modo que cualquier warning futura rompa el lint.

## Ficheros implicados

- `frontend/src/useConfigSync.ts` — nuevo: hook de sincronización desde props.
- `frontend/src/components/TelegramCard.tsx`, `ValetCard.tsx`, `MatrixCard.tsx` — usan el hook en lugar del `useEffect`.
- `frontend/src/components/DashboardPage.tsx` — reseteos de logs movidos a `handleLogs`.
- `frontend/src/App.tsx` — disable justificado del falso positivo asíncrono.
- `frontend/package.json` — script `lint` con `--deny-warnings`.
- Tests: `ConfigPage.test.tsx`, `DashboardPage.test.tsx`, `useConfigSync.test.ts`.

## Cómo funciona

- **Tres `*Card`:** el `useEffect` que sincronizaba el formulario desde `configProp` se sustituye por el hook `useConfigSync`, que aplica el patrón "adjusting state when a prop changes" de React (guarda la prop anterior en estado y compara durante el render). Se elimina el `useEffect` y la duplicación entre las tres tarjetas. Efecto lateral positivo: la sincronización ocurre antes del primer paint, sin flash de campos vacíos. El contrato del hook exige que `sync` sea idempotente y con setters del mismo componente.
- **`DashboardPage` logs:** los reseteos síncronos (`setLogs([])`, `setLogError(null)`, `setLogTimeout(false)`) se mueven del `useEffect` al event handler `handleLogs`. El efecto queda solo con el `EventSource` y el `setTimeout` (sistema externo), que es su uso legítimo.
- **`App.tsx`:** `fetchCheckConfig` hace `setState` después de `await`; la regla lo marca como falso positivo. Se documenta con `oxlint-disable-next-line react/set-state-in-effect` y un comentario justificativo (verificado: al quitarlo, oxlint vuelve a avisar).
- **Script `lint`:** `oxlint` → `oxlint --deny-warnings`. Verificado que un warning futuro hace fallar el comando con exit 1.

## Variables/config

Ninguna. El único cambio de configuración es el script `lint` en `frontend/package.json`.

## Cómo verificar

```bash
cd frontend
pnpm run test        # 196 tests
pnpm exec tsc -b
pnpm run lint        # 0 warnings
pnpm run build
```

## Decisiones

- Se usa el patrón canónico de React en lugar de `key` en el componente: `key` remontaría las tarjetas y perdería el estado de guardado en curso; el patrón de ajuste en render conserva el estado y sincroniza solo lo que depende de la prop.
- El falso positivo de `App.tsx` se documenta con disable en lugar de reestructurar el efecto: ni un IIFE async ni `void` evitan que la regla siga la llamada.
- `--deny-warnings` se activa ahora porque el repo queda con 0 warnings; evita que la deuda vuelva a acumularse.

## Cambio de comportamiento a tener en cuenta

Si el usuario pulsa "Logs" sobre el MISMO contenedor que ya está abierto, ahora se resetean logs, error y timeout (antes se mantenían porque el `useEffect` no se re-ejecutaba con la misma dependencia). Es coherente con el requisito "resetear al abrir" y evita mostrar logs rancios.

## Follow-ups

Ninguno pendiente. Los warnings de chunk >500 kB de Vite son preexistentes y ajenos a este change.
