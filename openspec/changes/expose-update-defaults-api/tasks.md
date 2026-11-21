# Tasks — expose-update-defaults-api

## 1. RED (tests que fallan)

- [ ] 1.1 Backend: test de `GET /api/update-check/defaults` ⇒ 200 con `cron="0 6 * * *"`,
      `pull_timeout_secs=600`, `check_interval_ms=2000`.
- [ ] 1.2 Frontend: test de que el placeholder/etiqueta/fallback usan los valores devueltos por
      `GET /api/update-check/defaults` (mock con valores **distintos** de los literales para que sea
      probatorio).

## 2. GREEN

- [ ] 2.1 `models.rs`: struct `UpdateDefaults { cron: String, pull_timeout_secs: u64, check_interval_ms: u64 }` (serde) + constructor desde consts.
- [ ] 2.2 `admin.rs`: `get_update_defaults_h` + ruta `GET /api/update-check/defaults` en `routes()`.
- [ ] 2.3 `types.ts`: tipo `UpdateDefaults`.
- [ ] 2.4 `ConfigPage.tsx`: cargar defaults; usarlos en placeholder cron, etiqueta timeout y fallback al
      vaciar; eliminar los literales `0 6 * * *` / `600` de esos usos.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` + `cargo test` en verde.
- [ ] 3.2 `cd frontend && pnpm exec tsc -b` + `pnpm run build` + tests en verde.
- [ ] 3.3 Docs: `docs/expose-update-defaults-api.md`.
- [ ] 3.4 Ajustar el spec archivado correspondiente (el delta `MODIFIED` lo cubre al archivar).

## 4. Archive

- [ ] 4.1 `openspec archive expose-update-defaults-api` tras merge.
