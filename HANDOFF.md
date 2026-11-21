# HANDOFF

## Estado

Última release: **v0.26.0** (`ghcr.io/atareao/alloy:0.26.0` y `:latest`); `main` **= `0d63204`**.
`develop` va por delante de `main`. Backlog de deuda: issues **#155–#166** (cerrados: #155, #157, #164).
**#156 tiene pendiente solo la ops manual en co1.**

### Completado esta sesión (repo)

- **#156** deploy co1 reproducible → **PR #168** mergeado + change archivado (`6b746d1`). Pendiente ops co1 (abajo).
- **#157** descomponer `ConfigPage.tsx` → **PR #169** mergeado + change `decompose-config-page` archivado (`ccfcec3`).
  - `ConfigPage.tsx` 1698→116 líneas (orquestador); módulo puro `src/cronValidation.ts`; hooks `useConfigPage`,
    `useUpdateCheckConfig`, `useDefaultPolicy`; 8 Cards + 3 tabs en `src/components/`. Suite `ConfigPage.test.tsx`
    intacta: 135 OK, `tsc -b`/`lint`/`build` limpios; auditoría `react-reviewer` sin bloqueantes.
  - Spec: `openspec/specs/ui/config-page/spec.md`. Doc: `docs/decompose-config-page.md`. Plan: `plans/PLAN-013-ARCHIVED.md`.

### Pendiente (ops manual, fuera del repo) — NO hecho

- **#156 ops en co1**: editar `compose.yaml` a `ghcr.io/atareao/alloy:0.26.0` + `docker compose up -d`; verificar
  `docker inspect` (tag) y HTTP 200. Cierra #156 al completarlo.

### Backlog pendiente (sin empezar)

- Orden: **#158 → #159 → #160 → #161 → #162 → #163 → #166**.

## Baselines de verificación

- Backend: `cargo test` → **459 OK** (12 ignored); `just check` limpio.
- Frontend: `npx vitest run` → **135 OK**; `pnpm exec tsc -b`, `lint`, `build` limpios.

## Entorno

- **co1** (Dockge) en `https://alloy.territoriolinux.es` — corre v0.26.0, sano. Procedimiento: `docs/deploy-co1.md`.
