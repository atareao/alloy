# HANDOFF

## Estado

Última release: **v0.26.0** (`ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` al día tras fusionar #182, #183 y #184 y va por delante de `main`. Backlog de deuda **#155–#170**: **todos cerrados** (no quedan issues abiertos).

### Completado esta sesión (repo)

- **#160** (README) mergeado y cerrado → **PR #174** (merge `d208c52`).
- **#161** (CI cache) mergeado y cerrado → **PR #175** (merge `4ee0273`).
- **#162** `frontend-test-coverage` implementado, verificado y publicado en **PR #176** (base `develop`).
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
- **#163 fase 2** `deps-major` → **PR #180** (`86ae6e1`), issue #163 cerrado.
  - Majors: `vitest 5.1.1`, `jsdom 31.0.1`, `typescript 7.0.2`, `@testing-library/jest-dom 8.0.0`; `engines.node ^24.15`; ajuste en `test-setup.ts` (`@testing-library/jest-dom/vitest`).
  - **174 tests**. Change archivado; `plans/PLAN-021-ARCHIVED.md`; `docs/frontend-deps.md`, `docs/security-dependencies.md`.
- **#166 parte 1** `container-row-ellipsis` → **PR #181**, issue #166 cerrado junto con la parte 2.
  - Elipsis real con tooltip, `minWidth: 0`, `flexShrink: 0`.
  - **179 tests**. Change archivado; `plans/PLAN-022-ARCHIVED.md`; `docs/container-row-ellipsis.md`.
- **#166 parte 2** `container-list-pagination` → **PR #182** (`0c4468d`).
  - Paginación y filtrado en servidor (REST + SSE + long-poll), contrato `{items,total,page,page_size}`, `state` repetible (OR), orden determinista, `summary` global.
  - Frontend: filtros en `App`, `containerQuery.ts`, debounce, reconexión del stream, `Pagination` 10/20/50, colapso al paginar.
  - **539 tests backend + 192 frontend**. Auditorías `@rust-reviewer` y `@react-reviewer`: **apto**.
  - Change archivado; `plans/PLAN-023-ARCHIVED.md`; `docs/container-list-pagination.md`.
- **#179** `lint-set-state-in-effect` → **PR #183** (`d464ac5`).
  - Warnings `react(set-state-in-effect)` resueltos; hook `useConfigSync`; `--deny-warnings` activado.
  - **201 tests**. Auditoría `@react-reviewer`: **apto**. Change archivado; `plans/PLAN-024-ARCHIVED.md`; `docs/lint-set-state-in-effect.md`.
- **#170** `update-history-lock-order` → **PR #184** (`23771a9`).
  - `append_stack_history` y `delete_history_h` liberan el guard antes de la pool; `record_update_entry` eliminado.
  - **541 tests**. Auditoría `@rust-reviewer`: **apto**. Change archivado; `plans/PLAN-025-ARCHIVED.md`; `docs/update-history-lock-order.md`.

### Pendiente

- **Ops manual**: verificar en `co1` que la SPA carga con la CSP aplicada.

### Follow-ups / deuda conocida

- Mostrar la imagen del contenedor dentro de `InspectModal` (la parte 1 la dejó fuera de la fila).
- `oxlint` ya con `--deny-warnings`; vigilar que no aparezcan warnings nuevas.
- Duplicado upstream de `@ant-design/cssinjs` (2.1.2 + 2.2.0) introducido por antd 6.7.1.

### Backlog pendiente (sin empezar)

- No quedan issues abiertos: backlog de deuda **#155–#170** cerrado.

## Baselines de verificación

- Backend: `cargo test` → **541 OK**; `just check` limpio.
- Frontend: `npx vitest run` → **201 OK**; `pnpm exec tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.

## Notas

- No quedan issues abiertos; los PRs se fusionaron con `--merge --delete-branch`; los issues se cerraron con `gh issue close`.
