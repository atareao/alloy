# HANDOFF

## Estado

Última release: **v0.26.0** (tag + imagen `ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main`. Backlog de deuda técnica: issues **#155–#166** (cerrados: #155, #156 repo, #164; #156 pendiente solo de ops).

### Completado esta sesión (repo)

- **#156** deploy co1 reproducible → **PR #168 mergeado** + **change archivado** (commit `6b746d1` en `develop`).
  - `docs/deploy-co1.md` corregido (imagen pinchada `:X.Y.Z` + verificación **por tag**), `deploy/co1.compose.example.yaml`,
    `docs/pin-deploy-co1-version.md`; spec `openspec/specs/dev/deploy-co1`. Plan: `plans/PLAN-012-ARCHIVED.md`.

### Pendiente (ops manual, fuera del repo) — NO hecho

- **#156 ops en co1**: editar `compose.yaml` a `ghcr.io/atareao/alloy:0.26.0` + `docker compose up -d`; verificar
  `docker inspect` (tag) y HTTP 200 en `https://alloy.territoriolinux.es/`. Issue #156 sigue abierto hasta esto.

### Backlog pendiente (sin empezar)

- **Siguiente: #157** (descomponer `ConfigPage.tsx`, 1698 líneas). Luego #158, #159, #160, #161, #162, #163, #166.

## Baselines de verificación

- Backend: `cargo test` → **459 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.
