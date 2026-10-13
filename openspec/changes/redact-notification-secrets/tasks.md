# Tasks

## 1. RED — Tests que fallan

- [ ] 1.1 `config.rs`: `GET /api/config` no incluye los tres tokens en claro; sí los
      `*_configured`.
- [ ] 1.2 `config.rs`: `PUT` con campo de token ausente conserva el actual; con `""` lo borra.
- [ ] 1.3 `ConfigPage.test.tsx`: el campo de token empieza vacío aunque `*_configured` sea
      `true`; guardar sin tocar el campo NO envía el token; escribir uno lo envía; desactivar
      envía `""`.
- [ ] 1.4 `cargo test` / `npx vitest run` en rojo con legacy verde.

## 2. GREEN — Implementación mínima

- [ ] 2.1 `models.rs`: quitar `telegram_token`/`matrix_token`/`valet_token` de `PublicConfig`.
- [ ] 2.2 `config.rs`: `config_handler` no rellena los tokens; mantiene los `*_configured`.
- [ ] 2.3 `frontend/src/types.ts`: quitar los tres campos de `AppConfig`.
- [ ] 2.4 `ConfigPage.tsx`: token efímero (vacío al cargar, placeholder "•••• configurado");
      `saveTelegram`/`saveMatrix`/`saveValet` omiten el token si el campo está vacío y el
      canal sigue activado; envían `""` si se desactiva; tras guardar, limpian el campo.
- [ ] 2.5 `cargo test` verde + `npx tsc -b` + `npx vitest run` verde.

## 3. REFACTOR — Verificación

- [ ] 3.1 `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] 3.2 `npm run lint` sin errores.
- [ ] 3.3 Revisión con `rust-reviewer` y `react-reviewer`.

## 4. Archive

- [ ] 4.1 `openspec validate redact-notification-secrets --strict`
- [ ] 4.2 `openspec archive redact-notification-secrets --yes`
- [ ] 4.3 Verificar/eliminar `openspec/changes/redact-notification-secrets/`
