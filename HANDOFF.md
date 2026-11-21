# HANDOFF

## Estado

Última release: **v0.26.0** (`ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main`. Backlog de deuda: issues **#155–#166** + **#170** (cerrados: #155, #156, #157, #158, #164).

### Completado esta sesión (repo)

- **#158** `settings-lock-scope` → **PR #171** (base `develop`) mergeado + change archivado.
  - Orden "conexión antes que estado" en 5 handlers (`config.rs::update_config_h`;
    `admin.rs::{put_update_check_config_h, put_default_update_policy_h, export_config_h, import_config_h}`):
    ya no se retiene el mutex de `Settings` durante `db_pool.get().await`.
  - 5 tests deterministas del invariante (pool `max_size=1` saturada + `timeout`) y helper endurecido;
    `drop(conn)` en `update_config_h`. Backend `cargo test` → **464 OK** (12 ignored); `just check` limpio.
  - Spec: `openspec/specs/backend/concurrency/spec.md` (capability `backend/concurrency`).
    Doc: `docs/settings-lock-scope.md`. Plan: `plans/PLAN-014-ARCHIVED.md`.
  - Auditoría `@rust-reviewer`: **apto**. Residual pool↔`update_history` en `updates/handlers.rs`
    fuera de alcance → **issue #170**.

### Pendiente (ops manual, fuera del repo)

- **#156 ops en co1** — ✅ HECHO: `compose.yaml` pinchado a `ghcr.io/atareao/alloy:0.26.0` + `docker compose up -d`; verificado tag por `docker inspect` y HTTP 200. Issue #156 cerrado.

### Backlog pendiente (sin empezar)

- Orden: **#159 → #160 → #161 → #162 → #163 → #166**. Follow-up abierto: **#170**
  (unificar orden *pool → estado* en `updates/handlers.rs`).

## Baselines de verificación

- Backend: `cargo test` → **464 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `pnpm exec tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.
