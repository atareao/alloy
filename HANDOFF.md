# HANDOFF

## Estado

Última release: **v0.26.0** (tag + imagen `ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main` (commits de docs/openspec/backlog + fixes ya mergeados; se integrarán en el
próximo release). Backlog de deuda técnica: issues **#155–#166** (dos ya cerrados: #164, #155).

### Completado esta sesión

- **#164** path traversal en el servido de estáticos → **PR #165** mergeado.
  - `safe_static_path(root, request_path)` (puro): canonicaliza y exige prefijo `dist/` + `is_file`; rechaza
    `..`, absolutos, `%2e%2e`, `\0` y symlinks que escapan. `serve_static` extraído (testeable con `tempdir`);
    fallback SPA intacto; sin `unwrap` en producción. Spec `openspec/specs/security/static-serving`.
    Doc: `docs/fix-static-path-traversal.md`; `plans/PLAN-010-ARCHIVED.md`.
- **#155** overrides de la guarda de la política por defecto → **PR #167** mergeado.
  - `Settings.default_max_start_attempts`/`default_health_window_secs` (Option, `serde default`) + helper
    `default_guard_overrides()`; `DefaultUpdatePolicy` (GET/PUT) los persiste/devuelve; `db.rs` key/value.
    `Settings::fallback_update_policy()` unifica el seeding de los 3 sitios (scheduler + handlers ×2).
    Precedencia `contenedor > default > global`. Spec `openspec/specs/updates/update-health-guard`.
    Doc: `docs/apply-default-guard-overrides.md`; `plans/PLAN-011-ARCHIVED.md`.

### Backlog pendiente (sin empezar)

Orden acordado: **#156 → #157 → #158 → #159 → #160 → #161 → #162 → #163 → #166**.

| # | Qué |
|---|---|
| #156 | 📝 ops: deploy co1 no reproducible (`:latest`) + doc desactualizada (sí hay tag por versión) |
| #157 | ♻️ refactor(ui): descomponer `ConfigPage.tsx` (1698 líneas) |
| #158 | ⚡ perf(backend): no retener `settings.lock()` durante `db_pool.get().await` |
| #159 | 🛡️ hardening(backend): `unwrap`→`AppError`, cookie `Secure`, headers, token por query |
| #160 | 📝 docs: README desactualizado (React 18/Mantine/JSON vs React 19/AntD 6/SQLite) |
| #161 | 👷 ci: `rust-cache` en `ci.yml`/`release.yml` |
| #162 | ✅ test(frontend): `DashboardPage`/`Container*`; recolocar `ErrorBoundary.test.tsx` |
| #163 | ⬆️ deps (frontend majors: vitest 5, jsdom 31, TS 7…) |
| #166 | ✨ ui: ellipsis en nombre/imagen del contenedor + paginación de la lista (10/20/50) |

## Baselines de verificación

- Backend: `cargo test` → **459 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `tsc -b`, `lint`, `build` limpios.

## Tarea pendiente

- **Siguiente: #156** (deploy co1 reproducible + doc). Change OpenSpec → aprobación → TDD → PR a `develop`.
  Es docs/procedimiento (probablemente sin código). Luego #157, #158…

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`
  (⚠️ #156: pinchar a `:0.26.0` en vez de `:latest`).
