# HANDOFF

## Estado

Última release: **v0.26.0** (`ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main`. Backlog de deuda: **#155–#166** + **#170** (cerrados: #155, #156, #157, #158, #164).

### Completado esta sesión (repo)

- **#159 punto 4** `sse-token-auth` → **PR #173** (base `develop`) + archivado.
  - `GET /api/auth/sse-token` emite un ticket firmado de 60 s con `purpose="sse"` y `Cache-Control: no-store`.
  - El middleware sólo acepta `?token=` en `GET /api/stream` y sólo como ticket SSE; rechaza el JWT de sesión por query, el ticket fuera del stream, expirado, o por cookie/Bearer.
  - La validación de sesión rechaza tokens con `purpose`; cookie/Bearer siguen funcionando.
  - Spec: `openspec/specs/auth/sse-ticket/spec.md`. Docs: `docs/architecture.md`, `docs/backend-hardening.md`. Plan: `plans/PLAN-016-ARCHIVED.md`.
  - Auditoría `@rust-reviewer`: **apto** (2 rondas; hallazgos no bloqueantes corregidos). Backend `cargo test` → **492 OK**.

### Pendiente

- **PR #173** en revisión/merge a `develop`.
- **Ops manual**: verificar en co1 que la SPA carga bien con la CSP aplicada (tras desplegar).

### Backlog pendiente (sin empezar)

- Orden: **#160 → #161 → #162 → #163 → #166**. Follow-ups abiertos: **#170**, **#159 punto 4** (PR #173).

## Baselines de verificación

- Backend: `cargo test` → **492 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `pnpm exec tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.
