# Tasks

## 1. RED — Tests que fallan

- [ ] 1.1 `config.rs` (función pura o handler): `telegram_configured` requiere token **y**
      chat_id; `matrix_configured` requiere homeserver **y** token **y** room; `valet_configured`
      requiere url **y** token; `webhook_configured` requiere url. Casos parciales ⇒ `false`.
- [ ] 1.2 `ConfigPage.test.tsx`: con config parcial (p. ej. homeserver sin token) el switch
      está on y se muestra el aviso "Faltan campos"; con config completa no hay aviso.
- [ ] 1.3 `cargo test` / `npx vitest run` en rojo con legacy verde.

## 2. GREEN — Implementación mínima

- [ ] 2.1 `config.rs`: derivar los `*_configured` con la conjunción de campos.
- [ ] 2.2 `ConfigPage.tsx`: inicializar `tgEnabled`/`mxEnabled`/`valetEnabled` con
      `configured || algún campo presente`, y mostrar el aviso si `enabled && !configured`.
- [ ] 2.3 `cargo test` verde + `npx tsc -b` + `npx vitest run` verde.

## 3. REFACTOR — Verificación

- [ ] 3.1 `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] 3.2 `npm run lint`; `cargo test`/`npx vitest run` sin regresiones.
- [ ] 3.3 Revisión con `rust-reviewer` y `react-reviewer`.

## 4. Archive

- [ ] 4.1 `openspec validate notify-configured-consistency --strict`
- [ ] 4.2 `openspec archive notify-configured-consistency --yes`
- [ ] 4.3 Verificar/eliminar `openspec/changes/notify-configured-consistency/`
