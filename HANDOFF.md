# HANDOFF

## Estado

Última release: **v0.26.0** (tag + imagen `ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` = **`8d48f63`** (6 commits por delante de `main`, solo docs/openspec/backlog — se integrarán en el
próximo release). Backlog de mejoras abierto tras una revisión de deuda técnica: issues **#155–#164**.

### Completado esta sesión — #164 (seguridad)
- **#164** path traversal en el servido de estáticos → **PR #165** mergeado a `develop`.
  - `safe_static_path(root, request_path)` (puro): canonicaliza y exige prefijo `dist/` + `is_file`; rechaza
    `..`, absolutos, `%2e%2e`, `\0` y **symlinks que escapan**. `serve_static` extraído (testeable con
    `tempdir`); **fallback SPA** intacto; sin `unwrap` en producción.
  - 15 tests nuevos; auditoría `@rust-reviewer` **APROBADA** (escape imposible verificado empíricamente).
  - Change OpenSpec archivado → spec `openspec/specs/security/static-serving`. Issue #164 cerrado.
  - `plans/PLAN-010.md` → `plans/PLAN-010-ARCHIVED.md`. Doc: `docs/fix-static-path-traversal.md`.

### Backlog pendiente (sin empezar)

Orden sugerido por impacto/esfuerzo: **#155 → #156 → #158 → #157 → #159 → #161 → #160 → #162 → #163**.

| # | Qué |
|---|---|
| #155 | 🐛 overrides de la guarda en la política por defecto se descartan (no-op silencioso) |
| #156 | 📝 deploy co1 no reproducible (`:latest`) + doc desactualizada (sí hay tag por versión) |
| #157 | ♻️ descomponer `ConfigPage.tsx` (1698 líneas) |
| #158 | ⚡ no retener `settings.lock()` durante `db_pool.get().await` |
| #159 | 🛡️ hardening backend (`unwrap`→`AppError`, cookie `Secure`, headers, token por query) |
| #160 | 📝 README desactualizado (React 18/Mantine/JSON vs React 19/AntD 6/SQLite) |
| #161 | 👷 CI sin `rust-cache` en `ci.yml`/`release.yml` |
| #162 | ✅ tests frontend (`DashboardPage`/`Container*`; recolocar `ErrorBoundary.test.tsx`) |
| #163 | ⬆️ deps (frontend majors: vitest 5, jsdom 31, TS 7…) |
| #166 | ✨ ui: ellipsis en nombre/imagen del contenedor + paginación de la lista (10/20/50) |

## Baselines de verificación

- Backend: `cargo test` → **447 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `tsc -b`, `lint`, `build` limpios.

## Tarea pendiente

- **Ninguna en curso.** Próximo ciclo: elegir del backlog (recomendado **#155**, bug funcional; o **#156**,
  S y reproducible). Cada ítem: issue ya existe → change OpenSpec → aprobación → TDD → auditoría → PR a
  `develop`. Release solo por merge `develop` → `main`.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`
  (⚠️ pendiente #156: pincharla a `:0.26.0` en vez de `:latest`).
