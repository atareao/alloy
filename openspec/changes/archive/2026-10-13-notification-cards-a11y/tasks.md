# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 `ConfigPage.test.tsx`: obtener por nombre accesible los switches "Activar Telegram"
      y "Activar Matrix" y los inputs "Token del bot de Telegram" y "Access token de Matrix".
- [x] 1.2 `npx vitest run` en rojo.

## 2. GREEN — Implementación mínima

- [x] 2.1 `ConfigPage.tsx`: `aria-label` en los dos `<Switch>` y en los dos inputs de token.
- [x] 2.2 `npx tsc -b` + `npx vitest run` verde.

## 3. REFACTOR — Verificación

- [x] 3.1 `npm run lint` sin errores.
- [x] 3.2 `npx vitest run` sin regresiones.
- [x] 3.3 Revisión con `react-reviewer`.

## 4. Archive

- [x] 4.1 `openspec validate notification-cards-a11y --strict`
- [x] 4.2 `openspec archive notification-cards-a11y --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/notification-cards-a11y/`
