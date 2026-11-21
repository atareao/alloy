# Tasks — reconcile-update-defaults

## 1. RED (tests que fallan)

- [ ] 1.1 Backend: test de que `Settings`/`UpdateCheckConfig` sin cron ⇒ `0 6 * * *`.
- [ ] 1.2 Backend: test de que sin `pull_timeout_secs` ⇒ `600` (en la ruta que corresponda).
- [ ] 1.3 Backend: test de que sin `check_interval_ms` ⇒ `2000`.
- [ ] 1.4 Frontend: `ConfigPage.test.tsx` ajustado a `0 6 * * *` y `600` (falla con los valores actuales).

## 2. GREEN

### 2.1 Backend — único origen
- [ ] 2.1.1 `models.rs`: consts `DEFAULT_UPDATE_CHECK_CRON`, `DEFAULT_PULL_TIMEOUT_SECS`,
      `DEFAULT_CHECK_INTERVAL_MS` + uso en `Default`/serde.
- [ ] 2.1.2 `db.rs`: fallbacks de lectura → consts.
- [ ] 2.1.3 `admin.rs`: GET fallback de cron → const.
- [ ] 2.1.4 `workers/scheduler.rs`: 3 fallbacks de timeout + fallback de check_interval + fallback de
      cron → consts.
- [ ] 2.1.5 `updates/handlers.rs`: 3 fallbacks de timeout + check_interval + cron de timestamps → consts.
- [ ] 2.1.6 `stacks.rs`: fallback de timeout → const.

### 2.2 Frontend — alineación
- [ ] 2.2.1 `ConfigPage.tsx`: cron inicial/placeholder → `0 6 * * *`; `pull_timeout_secs` inicial/fallback
      → `600`; etiqueta de UI → `600 (10 min)`.
- [ ] 2.2.2 Ajustar presets/placeholder coherentes.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` + `cargo test` en verde.
- [ ] 3.2 `cd frontend && pnpm run build` + `pnpm exec tsc -b` + lint/tests en verde.
- [ ] 3.3 Verificar por `rg` que no quedan literales duplicados (`"0 */6 * * *"`, `"0 0 * * *"`,
      `unwrap_or(600)`, `unwrap_or(1800)`, `unwrap_or(2000)`) en producción.
- [ ] 3.4 Docs: `docs/reconcile-update-defaults.md`.

## 4. Archive

- [ ] 4.1 `openspec archive reconcile-update-defaults` tras merge.
