# Tasks — expose-guard-defaults-api

## 1. RED (tests que fallan)

- [x] 1.1 `backend/src/admin.rs`: el test del endpoint `GET /api/update-check/defaults` exige también
      `update_max_start_attempts = 3`, `update_start_attempt_interval_secs = 10`,
      `update_health_window_secs = 30`.
- [x] 1.2 `frontend/src/components/ConfigPage.test.tsx`: con un mock de `/defaults` de valores **distintos**
      de los canónicos (p. ej. `update_max_start_attempts: 5`), vaciar el campo de la guarda aplica el
      valor de la API (no el literal `3`).

## 2. GREEN

- [x] 2.1 `backend/src/models.rs`: ampliar `UpdateDefaults` con los 3 campos de la guarda + poblarlos en
      `canonical()` desde las consts.
- [x] 2.2 `frontend/src/types.ts`: ampliar `interface UpdateDefaults` con los 3 campos.
- [x] 2.3 `frontend/src/components/ConfigPage.tsx`: usar `defaults?.<campo>` en el fallback de los 3 campos
      de la guarda; eliminar los literales `3/10/30` de esos usos.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` + `cargo test` (sin regresiones).
- [ ] 3.2 `cd frontend && pnpm exec tsc -b` + `pnpm run lint` + `pnpm run build` + tests en verde.
- [ ] 3.3 Docs: `docs/expose-guard-defaults-api.md`.

## 4. Archive

- [ ] 4.1 `openspec archive expose-guard-defaults-api` tras merge.
