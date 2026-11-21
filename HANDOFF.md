# HANDOFF

## Estado

Última release: **v0.26.0** (`ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main`. Backlog de deuda: **#155–#166** + **#170** (cerrados: #155, #156, #157, #158, #164).

### Completado esta sesión (repo)

- **#158** `settings-lock-scope` → PR #171 mergeado + archivado (spec `backend/concurrency`).
- **#159 (puntos 1–3)** `backend-hardening` → **PR #172** (base `develop`) + archivado.
  - `AppError::from_pool`; handlers dependientes de la conexión → `Result<Json<T>, AppError>` (500);
    best-effort con `tracing::warn!`; `fetch_containers` propaga el error (no `200 []`).
  - Cookie `Secure` por defecto (opt-out `ALLOW_INSECURE_COOKIES=1`).
  - Cabeceras de seguridad en `build_app` (nosniff, DENY, Referrer-Policy, CSP AntD, HSTS HTTPS).
  - Specs: `openspec/specs/backend/error-handling/spec.md`, `openspec/specs/security/headers/spec.md`,
    delta `auth/session`. Docs: `docs/backend-hardening.md`. Plan: `plans/PLAN-015-ARCHIVED.md`.
  - Auditoría `@rust-reviewer`: **apto** (3 hallazgos corregidos). Backend `cargo test` → **470 OK**.

### Pendiente

- **#159 punto 4** (token por `?token=`/SSE) → change/PR propio (siguiente).
- **Ops manual**: verificar en co1 que la SPA carga bien con la CSP aplicada (tras desplegar).

### Backlog pendiente (sin empezar)

- Orden: **#160 → #161 → #162 → #163 → #166**. Follow-ups abiertos: **#170**, **#159 punto 4**.

## Baselines de verificación

- Backend: `cargo test` → **470 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `pnpm exec tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.
