# Tasks — apply-default-guard-overrides

## 1. RED (tests que fallan)

- [ ] 1.1 `admin.rs`: round-trip `PUT`/`GET /api/update-policies/default` con
      `max_start_attempts`/`health_window_secs` (persisten en `Settings` y se devuelven);
      y `PUT` con `null` los limpia a `None`.
- [ ] 1.2 `models.rs`/`handlers.rs`: un contenedor **sin** política resuelve la guarda con el
      override por defecto (no el global); la política por contenedor sigue ganando; una
      política sin override cae al global.
- [ ] 1.3 Compatibilidad: un JSON de `Settings` sin los campos nuevos deserializa a `None`.

## 2. GREEN

- [ ] 2.1 `models.rs`: `Settings.default_max_start_attempts`/`default_health_window_secs`
      (`#[serde(default)]`) + en `Default`; helper `Settings::default_guard_overrides()`.
- [ ] 2.2 `admin.rs`: `DefaultUpdatePolicy` con los dos `Option`; `GET` los puebla; `PUT` los persiste.
- [ ] 2.3 Sembrar la `UpdatePolicy` de *fallback* con `settings.default_guard_overrides()` en
      `workers/scheduler.rs` y `updates/handlers.rs` (los 3 sitios).

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` + `cargo test` en verde.
- [ ] 3.2 Docs: `docs/apply-default-guard-overrides.md`.

## 4. Archive

- [ ] 4.1 `openspec archive apply-default-guard-overrides` tras merge.
