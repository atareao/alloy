# Tasks

## TDD Checklist

### RED

- [x] Añadir tests que fijen el comportamiento actual de precarga de campos, token vacío, switch activado y logs (`ConfigPage.test.tsx`, `DashboardPage.test.tsx`) si no están cubiertos.
- [x] Ejecutar `pnpm run test` y confirmar el estado de partida.

### GREEN

- [x] Crear `frontend/src/useConfigSync.ts` con el patrón de ajuste durante el render.
- [x] Refactorizar `TelegramCard.tsx`, `ValetCard.tsx` y `MatrixCard.tsx` para usar el hook y eliminar el `useEffect`.
- [x] Mover los reseteos de logs a `handleLogs` en `DashboardPage.tsx` y dejar el `useEffect` solo con el `EventSource` y el timer.
- [x] Añadir el `oxlint-disable-next-line react/set-state-in-effect` justificado en `App.tsx`.
- [x] Cambiar el script `lint` a `oxlint --deny-warnings`.

### REFACTOR

- [x] Revisar duplicación restante entre los tres `*Card`.
- [x] Ejecutar `pnpm run lint` y confirmar 0 warnings.

### Verificación

- [x] `pnpm run test` → 0 (≥190 tests).
- [x] `pnpm exec tsc -b` → 0.
- [x] `pnpm run lint` → 0 sin warnings.
- [x] `pnpm run build` → 0.
- [ ] Commit con gitmoji (`refactor`) y PR a `develop`.
