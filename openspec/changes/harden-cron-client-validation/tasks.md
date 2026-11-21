# Tasks — harden-cron-client-validation

## 1. RED (tests que fallan)

- [x] 1.1 `ConfigPage.test.tsx`: `99 0 * * *` (fuera de rango) ⇒ error inline + `PUT` no disparado.
- [x] 1.2 `ConfigPage.test.tsx`: `0 0 * * 0` (día-de-semana `0`, inválido en la crate) ⇒ error + `PUT` no
      disparado.
- [x] 1.3 `ConfigPage.test.tsx`: `30-10 * * * *` (rango invertido), `*/0 * * * *` (paso cero) y
      `*/60 * * * *` (paso > máximo) ⇒ error + `PUT` no disparado.
- [x] 1.4 `ConfigPage.test.tsx`: válidos `*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *`, `0 6 * * 1-5`,
      `0 6 ? * *`, `0 6 * * MON-FRI`, `0 6 1,15 * *` ⇒ sin error + `PUT`.
- [x] 1.5 `ConfigPage.test.tsx`: nombres largos `0 6 * * MONDAY`, `0 6 1 JANUARY *` ⇒ sin error + `PUT`.
- [x] 1.6 `ConfigPage.test.tsx`: el preset «Cada domingo» produce un cron que pasa la validación cliente
      (día-de-semana `1–7`, no `0`).
- [x] 1.7 `ConfigPage.test.tsx`: adaptar el test "backend responde 400" a un cron **client-valid** (el mock
      simula el 400) manteniendo la aserción del mensaje del backend.

## 2. GREEN

- [x] 2.1 `ConfigPage.tsx`: endurecer `isValidCronClient`/`cronErrorFor` con validación por campo de
      rangos (día-de-semana `1–7`), listas, rangos `a-b`, `?` en día-del-mes/día-de-semana, pasos `1..=máx`
      y nombres (mes/día-de-semana, ≥3 letras), sin dependencias.
- [x] 2.2 `ConfigPage.tsx`: arreglar el preset «Cada domingo» `0 0 * * 0` → `0 0 * * SUN`.

## 3. REFACTOR / verificación

- [x] 3.1 `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests en verde.
- [x] 3.2 Docs: `docs/harden-cron-client-validation.md`.

## 4. Archive

- [ ] 4.1 `openspec archive harden-cron-client-validation` tras merge.
