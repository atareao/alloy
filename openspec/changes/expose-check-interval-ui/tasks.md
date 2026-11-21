# Tasks — expose-check-interval-ui

## 1. RED (tests que fallan)

- [ ] 1.1 `ConfigPage.test.tsx`: la config con `check_interval_ms = 2000` ⇒ el control del intervalo
      muestra `2` (segundos).
- [ ] 1.2 `ConfigPage.test.tsx`: poner `5` segundos y guardar ⇒ el `PUT` envía `check_interval_ms = 5000`.
- [ ] 1.3 `ConfigPage.test.tsx`: la etiqueta/placeholder del intervalo reflejan el default de
      `GET /api/update-check/defaults` (en segundos), sin literales.

## 2. GREEN

- [ ] 2.1 `frontend/src/types.ts`: añadir `check_interval_ms` a `UpdateCheckConfig`.
- [ ] 2.2 `ConfigPage.tsx`: estado `checkIntervalSecs`, carga (`/1000`), `InputNumber` (min 1 s) y envío
      (`* 1000`) en el `PUT`; etiqueta/placeholder desde `defaults.check_interval_ms / 1000`.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests en verde.
- [ ] 3.2 Docs: `docs/expose-check-interval-ui.md`.

## 4. Archive

- [ ] 4.1 `openspec archive expose-check-interval-ui` tras merge.
