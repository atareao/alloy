# Tasks

## 1. RED — Test que falla

- [x] 1.1 `admin.rs`: `export_config_h` NO incluye tokens en claro en `settings`
      (los tres a `null`) con settings que sí los tienen; el resto de campos y
      `update_history` intactos.
- [x] 1.2 `cargo test` en rojo con legacy verde.

## 2. GREEN — Implementación mínima

- [x] 2.1 `admin.rs::export_config_h`: clonar `Settings`, poner los tres tokens a `None` y
      serializarlo en `settings`.
- [x] 2.2 `ConfigPage.tsx`: advertir en el texto de export/import que los tokens no se
      exportan.
- [x] 2.3 `cargo test` verde + `npx tsc -b`.

## 3. REFACTOR — Verificación

- [x] 3.1 `cargo fmt` / `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.2 `cargo test` sin regresiones; `npx vitest run`.
- [x] 3.3 Revisión con `rust-reviewer`.

## 4. Archive

- [x] 4.1 `openspec validate redact-export-secrets --strict`
- [x] 4.2 `openspec archive redact-export-secrets --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/redact-export-secrets/`
