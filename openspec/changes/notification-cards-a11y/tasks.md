# Tasks

## 1. RED — Tests que fallan

- [ ] 1.1 `ConfigPage.test.tsx`: obtener por nombre accesible los switches "Activar Telegram"
      y "Activar Matrix" y los inputs "Token del bot de Telegram" y "Access token de Matrix".
- [ ] 1.2 `npx vitest run` en rojo.

## 2. GREEN — Implementación mínima

- [ ] 2.1 `ConfigPage.tsx`: `aria-label` en los dos `<Switch>` y en los dos inputs de token.
- [ ] 2.2 `npx tsc -b` + `npx vitest run` verde.

## 3. REFACTOR — Verificación

- [ ] 3.1 `npm run lint` sin errores.
- [ ] 3.2 `npx vitest run` sin regresiones.
- [ ] 3.3 Revisión con `react-reviewer`.

## 4. Archive

- [ ] 4.1 `openspec validate notification-cards-a11y --strict`
- [ ] 4.2 `openspec archive notification-cards-a11y --yes`
- [ ] 4.3 Verificar/eliminar `openspec/changes/notification-cards-a11y/`
