# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 `notifications.rs`: `valet_level()` — estados críticos (`died`/`dead`/`error`) →
      `critical`; parada (`exit`/`stop`) → `warning`; resto → `info`.
- [x] 1.2 `notifications.rs`: `notify_valet` devuelve `Err` sin url, sin token y sin ambos
      (early return, sin red).
- [x] 1.3 `notifications.rs`: `notify_valet` normaliza `valet_url` quitando `/` final
      (helper puro `valet_endpoint()`).
- [x] 1.4 `notifications.rs`: el endpoint de test con `channel:"valet"` no configurado
      devuelve error controlado (no panic).
- [x] 1.5 `cargo test` en rojo con los legacy en verde.

## 2. GREEN — Implementación mínima

- [x] 2.1 `models.rs`: `valet_url`/`valet_token` en `Settings` y `UpdateSettingsReq`;
      `valet_url`/`valet_token`/`valet_configured` en `PublicConfig`.
- [x] 2.2 `db.rs`: persistir y cargar `valet_url`/`valet_token` en `load_settings`/`save_settings`.
- [x] 2.3 `config.rs`: `config_handler` (derivar `valet_configured`) y `update_config_h`
      (cadena vacía ⇒ `None`) para ambos campos.
- [x] 2.4 `notifications.rs`: `valet_level()`, `valet_endpoint()`, `notify_valet()`
      (`POST {base}/api/notifications`, Bearer, JSON con message/origin/author/level/metadata).
- [x] 2.5 `notifications.rs`: añadir `notify_valet` a `notify_all` (`tokio::join!`).
- [x] 2.6 `notifications.rs`: canal `"valet"` en `test_notification_h`; actualizar el
      mensaje de canal desconocido.
- [x] 2.7 `frontend/src/types.ts`: campos Valet en `AppConfig`.
- [x] 2.8 `frontend/src/components/ConfigPage.tsx`: tarjeta Valet + `saveValet`.
- [x] 2.9 `cargo test` verde + `cargo check`; `npx tsc -b` + `npx vitest run`.

## 2b. Corrección de integración (hallazgo de review)

- [x] 2b.1 `workers/state.rs`: `detect_notifications` devuelve `(name, status_message, raw_state)`
      y sus tests se actualizan.
- [x] 2b.2 `notifications.rs`: `notify_all_state(settings, container, status, raw_state: Option<&str>)`
      calcula `level = raw_state.map(valet_level).unwrap_or("info")`; `notify_all` delega con
      `raw_state=None`; `notify_valet` recibe el `level` ya calculado.
- [x] 2b.3 `workers/state.rs`: el worker llama a `notify_all_state(..., &status_msg, Some(&raw_state))`.
- [x] 2b.4 Test: un contenedor `exited`/`dead` produce `warning`/`critical` end-to-end por el
      helper (no `info`), pese a que el mensaje visible esté localizado.
- [x] 2b.5 `cargo test` verde + clippy.

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 `cargo fmt`
- [x] 3.2 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 `npx eslint` + `npx prettier --check` en frontend
- [x] 3.4 `cargo test` / `npx vitest run` sin regresiones
- [x] 3.5 Revisión con `rust-reviewer` (backend) y `react-reviewer` (frontend)

## 4. Archive

- [x] 4.1 `openspec validate valet-notifier --strict`
- [x] 4.2 `openspec archive valet-notifier --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/valet-notifier/`
