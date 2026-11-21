# Tasks — validate-cron-on-put

## 1. RED (tests que fallan)

- [x] 1.1 `timezone.rs`: tests de `is_valid_cron` — 5 campos válidos ⇒ true; vacío / texto / 6 campos ⇒ false.
- [x] 1.2 `admin.rs`: test de `put_update_check_config_h` con cron inválido ⇒ **400** y settings SIN cambios.
- [x] 1.3 `admin.rs`: test con cron vacío ⇒ 400.
- [x] 1.4 `admin.rs`: test con cron de 6 campos ⇒ 400.
- [x] 1.5 `admin.rs`: test con cron válido ⇒ 200 (sin regresión del `test_put_update_check_config`).
- [x] 1.6 Frontend: `ConfigPage.test.tsx` — guardar con cron inválido y respuesta 400 ⇒ se muestra el `error` del backend.

## 2. GREEN

- [x] 2.1 `timezone.rs`: `pub fn is_valid_cron(expr: &str) -> bool` (5 campos + parseo con `"0 "`).
- [x] 2.2 `admin.rs`: `put_update_check_config_h` → `Result<Json<UpdateCheckConfig>, AppError>`; validar `body.cron` antes de mutar; `AppError::BadRequest` con mensaje claro.
- [x] 2.3 `ConfigPage.tsx`: `saveUpdateCheck` lee el body del error (`res.json().catch`) y muestra `data.error`.

## 3. REFACTOR / verificación

- [x] 3.1 `cd backend && just check` + `cargo test` en verde.
- [x] 3.2 `cd frontend && pnpm exec tsc -b` + `pnpm run build` + tests en verde.
- [x] 3.3 `rg` para confirmar que no se duplica el prefijado `"0 "`/parseo de cron en los puntos de validación.
- [ ] 3.4 Docs: `docs/validate-cron-on-put.md`.

## 4. Archive

- [ ] 4.1 `openspec archive validate-cron-on-put` tras merge.

## 5. Ampliación — validación del cron en import_config_h (aprobado)

- [x] 5.1 RED: `admin.rs` — `import_config_h` con `update_check_cron = Some("not a cron")` ⇒ **400** y
      estado (memoria + disco) sin cambios; con `None`/`Some("")` ⇒ 200. (también 6 campos ⇒ 400)
- [x] 5.2 GREEN: `import_config_h` → `Result<Json<serde_json::Value>, AppError>`; validar antes de mutar;
      delegar en `is_valid_cron` (solo si presente y no vacío).
