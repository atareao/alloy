# Tasks — reconcile-update-defaults

## 1. RED (tests que fallan)

- [x] 1.1 Backend: test de que `Settings`/`UpdateCheckConfig` sin cron ⇒ `0 6 * * *`.
- [x] 1.2 Backend: test de que sin `pull_timeout_secs` ⇒ `600` (en la ruta que corresponda).
- [x] 1.3 Backend: test de que sin `check_interval_ms` ⇒ `2000`.
- [x] 1.4 Frontend: `ConfigPage.test.tsx` ajustado a `0 6 * * *` y `600` (falla con los valores actuales).

## 2. GREEN

### 2.1 Backend — único origen
- [x] 2.1.1 `models.rs`: consts `DEFAULT_UPDATE_CHECK_CRON`, `DEFAULT_PULL_TIMEOUT_SECS`,
      `DEFAULT_CHECK_INTERVAL_MS` + helpers `effective_update_check_cron` /
      `effective_pull_timeout_secs` / `effective_check_interval_ms` en `Settings`.
- [x] 2.1.2 `db.rs`: fallbacks de lectura → consts (los 3 campos se leen como `Option`, sin literal que sustituir).
- [x] 2.1.3 `admin.rs`: GET fallback de cron → const.
- [x] 2.1.4 `workers/scheduler.rs`: 3 fallbacks de timeout + fallback de check_interval + fallback de
      cron → consts.
- [x] 2.1.5 `updates/handlers.rs`: 3 fallbacks de timeout + check_interval + cron de timestamps → consts.
- [x] 2.1.6 `stacks.rs`: fallback de timeout → const.

### 2.2 Frontend — alineación
- [x] 2.2.1 `ConfigPage.tsx`: cron inicial/placeholder → `0 6 * * *`; `pull_timeout_secs` inicial/fallback
      → `600`; etiqueta de UI → `600 (10 min)`.
- [x] 2.2.2 Ajustar presets/placeholder coherentes. (`CRON_PRESETS` conserva `0 */6 * * *` como preset
      seleccionable; el default y el placeholder ya apuntan al canónico `0 6 * * *`.)

## 3. REFACTOR / verificación

- [x] 3.1 `cd backend && just check` + `cargo test` en verde. (418 passed / 0 failed · clippy + fmt OK)
- [x] 3.1.bis Auditoría backend — observación 1: `effective_update_check_cron()` normaliza
      `Some("")` ⇒ default canónico (`filter(|s| !s.is_empty())`), misma semántica que `db.rs`.
      Test `models::tests::test_settings_empty_cron_uses_canonical_default`.
- [x] 3.1.ter Auditoría backend — observación 2: el GET de `/api/update-check/config` sirve
      `pull_timeout_secs`/`check_interval_ms` **efectivos** (`Some(effective_*)`). Ajustado
      `admin::tests::test_get_update_check_config_defaults`.
- [x] 3.2 `cd frontend && pnpm run build` + `pnpm exec tsc -b` + lint/tests en verde.
      (`tsc -b` 0 errores · `pnpm run build` OK · `oxlint` 0 · `vitest run` 101/101 en verde.)
- [x] 3.3 Verificar por `rg` que no quedan literales duplicados (`"0 */6 * * *"`, `"0 0 * * *"`,
      `unwrap_or(600)`, `unwrap_or(1800)`, `unwrap_or(2000)`) en producción. (Backend: los `unwrap_or`
      han desaparecido; los cron restantes están solo en `#[cfg(test)]`.)
- [ ] 3.4 Docs: `docs/reconcile-update-defaults.md`.

## 4. Archive

- [ ] 4.1 `openspec archive reconcile-update-defaults` tras merge.
