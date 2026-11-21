# Tasks — validate-cron-client-ui

## 1. RED (tests que fallan)

- [x] 1.1 `ConfigPage.test.tsx`: un cron de **6 campos** introducido y blur ⇒ el campo queda en 5 campos;
      guardar envía `cron` de 5 campos (sin 400).
- [x] 1.2 `ConfigPage.test.tsx`: cron **malformado** ⇒ error inline visible **y** `PUT` NO disparado
      (assert de que `fetch` no recibió el PUT).
- [x] 1.3 `ConfigPage.test.tsx`: cron válido de 5 campos ⇒ sin error inline y PUT disparado.

## 2. GREEN

- [x] 2.1 `ConfigPage.tsx`: `normalizeCron(expr)` (6→5 quitando segundos) e `isValidCronClient(expr)`
      (5 campos + charset), sin dependencias nuevas.
- [x] 2.2 Error inline (`cronError`) bajo el campo cron.
- [x] 2.3 Normalizar onBlur; `saveUpdateCheck` bloquea el PUT si inválido (normaliza antes de enviar).

## 3. REFACTOR / verificación

- [x] 3.1 `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests en verde.
- [ ] 3.2 Docs: `docs/validate-cron-client-ui.md`.

## 4. Archive

- [ ] 4.1 `openspec archive validate-cron-client-ui` tras merge.
