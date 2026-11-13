# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 `config.rs`: `GET /api/config` no incluye los tres tokens en claro; sí los
      `*_configured`.
- [x] 1.2 `config.rs`: `PUT` con campo de token ausente conserva el actual; con `""` lo borra.
- [x] 1.3 `ConfigPage.test.tsx`: el campo de token empieza vacío aunque `*_configured` sea
      `true`; guardar sin tocar el campo NO envía el token; escribir uno lo envía; desactivar
      envía `""`.
- [x] 1.4 `cargo test` / `npx vitest run` en rojo con legacy verde.

## 2. GREEN — Implementación mínima

- [x] 2.1 `models.rs`: quitar `telegram_token`/`matrix_token`/`valet_token` de `PublicConfig`.
- [x] 2.2 `config.rs`: `config_handler` no rellena los tokens; mantiene los `*_configured`.
- [x] 2.3 `frontend/src/types.ts`: quitar los tres campos de `AppConfig`.
- [x] 2.4 `ConfigPage.tsx`: token efímero (vacío al cargar, placeholder "•••• configurado");
      `saveTelegram`/`saveMatrix`/`saveValet` omiten el token si el campo está vacío y el
      canal sigue activado; envían `""` si se desactiva; tras guardar, limpian el campo.
- [x] 2.5 `cargo test` verde + `npx tsc -b` + `npx vitest run` verde.

## 3. REFACTOR — Verificación

- [x] 3.1 `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.2 `npm run lint` sin errores.
- [x] 3.3 Revisión con `rust-reviewer` y `react-reviewer`.

## 4. Archive

- [x] 4.1 `openspec validate redact-notification-secrets --strict`
- [x] 4.2 `openspec archive redact-notification-secrets --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/redact-notification-secrets/`
