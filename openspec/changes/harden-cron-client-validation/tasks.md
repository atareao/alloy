# Tasks — harden-cron-client-validation

## 1. RED (tests que fallan)

- [ ] 1.1 `ConfigPage.test.tsx`: `99 0 * * *` (fuera de rango) ⇒ error inline + `PUT` no disparado.
- [ ] 1.2 `ConfigPage.test.tsx`: `30-10 * * * *` (rango invertido) y `*/0 * * * *` (paso cero) ⇒ error +
      `PUT` no disparado.
- [ ] 1.3 `ConfigPage.test.tsx`: válidos `*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *`, `0 6 * * 1-5` ⇒ sin
      error + `PUT`.
- [ ] 1.4 `ConfigPage.test.tsx`: adaptar el test "backend responde 400" a un cron **client-valid**
      (el mock simula el 400) manteniendo la aserción del mensaje del backend.

## 2. GREEN

- [ ] 2.1 `ConfigPage.tsx`: endurecer `isValidCronClient`/`cronErrorFor` con validación por campo de
      rangos, listas, rangos `a-b`, pasos `/n` (>0) y nombres (mes/día), sin dependencias.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests en verde.
- [ ] 3.2 Docs: `docs/harden-cron-client-validation.md`.

## 4. Archive

- [ ] 4.1 `openspec archive harden-cron-client-validation` tras merge.
