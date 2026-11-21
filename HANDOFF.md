# HANDOFF

## Estado

Última release: **v0.26.0** (`ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main`. Backlog de deuda: **#155–#166** + **#170** (cerrados: #155, #156, #157, #158, #159, #160, #161, #162, #164).

### Completado esta sesión (repo)

- **#160** (README) mergeado y cerrado → **PR #174** (merge `d208c52`).
- **#161** (CI cache) mergeado y cerrado → **PR #175** (merge `4ee0273`).
- **#162** `frontend-test-coverage` implementado, verificado (174 tests frontend; `tsc`/`lint`/`build` limpios) y publicado en **PR #176** (base `develop`).
  - Change archivado: `openspec/specs/frontend/test-coverage/spec.md`. Doc: `docs/frontend-test-coverage.md`. Plan: `plans/PLAN-019-ARCHIVED.md`.
- **#159 punto 4** `sse-token-auth` → **PR #173** (base `develop`) + archivado.
  - `GET /api/auth/sse-token` emite un ticket firmado de 60 s con `purpose="sse"` y `Cache-Control: no-store`.
  - El middleware sólo acepta `?token=` en `GET /api/stream` y sólo como ticket SSE; rechaza el JWT de sesión por query, el ticket fuera del stream, expirado, o por cookie/Bearer.
  - La validación de sesión rechaza tokens con `purpose`; cookie/Bearer siguen funcionando.
  - Spec: `openspec/specs/auth/sse-ticket/spec.md`. Docs: `docs/architecture.md`, `docs/backend-hardening.md`. Plan: `plans/PLAN-016-ARCHIVED.md`.
  - Auditoría `@rust-reviewer`: **apto** (2 rondas; hallazgos no bloqueantes corregidos). Backend `cargo test` → **492 OK**.
- **#163 fase 1** `deps-minor` mergeado → **PR #177** (merge `9860f005`).
  - Bump de parches/menores sin cruzar majors: antd 6.7.1, react/react-dom 19.3.1, vite 8.3.8, oxlint 1.94, plugin-react 6.3.0, testing-library 16.3.3, types 19.3.0; jest-dom capado a `~6.9.1`.
  - Change archivado: `openspec/specs/frontend/deps/spec.md`. Doc: `docs/frontend-deps.md`. Plan: `plans/PLAN-020-ARCHIVED.md`.
  - Auditoría `@react-reviewer`: **apto**.

### Pendiente

- **#163 fase 2** (`deps-major`: vitest 5, jsdom 31, TypeScript 7, jest-dom 8) por hacer.
- **PR #176** en revisión/merge a `develop`.
- **Ops manual**: verificar en co1 que la SPA carga bien con la CSP aplicada (tras desplegar).

### Follow-ups / deuda conocida

- `oxlint` 1.94 activa 5 warnings preexistentes de `react(set-state-in-effect)` (`TelegramCard`, `ValetCard`, `MatrixCard`, `DashboardPage:199`, `App:326`); **no** añadir `--deny-warnings` hasta resolverlos.
- Duplicado upstream de `@ant-design/cssinjs` (2.1.2 + 2.2.0) introducido por antd 6.7.1.

### Backlog pendiente (sin empezar)

- Pendientes: **#166**, **#170** y **#163 fase 2** (`deps-major`). **#163 fase 1** (`deps-minor`) ya está hecha (PR #177).
- Orden sugerido: **#163 fase 2 → #166**. Follow-ups abiertos: **#170**.

## Baselines de verificación

- Backend: `cargo test` → **492 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **174 OK**; `pnpm exec tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.
