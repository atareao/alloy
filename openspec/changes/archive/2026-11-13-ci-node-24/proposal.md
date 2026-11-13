# Proposal

## Why

La automatización de CI/release acumula dos deudas de toolchain (issue #99), ambas visibles en
cada ejecución:

1. **Actions sobre Node 20 (deprecado).** Los runners ya fuerzan Node 24, así que cada run emite
   el warning *"Node.js 20 is deprecated. The following actions target Node.js 20 but are being
   forced to run on Node.js 24: actions/checkout@v4, actions/upload-artifact@v4, …"*.
2. **Node 23 (non-LTS, EOL)** en el `Dockerfile` (`node:23-alpine`) y en `ci.yml`
   (`node-version: 23`). El entorno local usa `24.22.1`. Hay **tres versiones** en juego y
   ninguna es una LTS soportada.

El propio `docs/frontend-ci-verification.md` (sección *Follow-ups*) ya identificaba ambos
puntos y proponía unificar en la LTS vigente con una **fuente única de verdad**.

## What Changes

- **Actions a Node 24** en los tres workflows, usando la **versión mayor mínima que ya corre en
  Node 24** (para no arrastrar cambios de majors más nuevos en un pipeline que no se puede
  probar en un PR):
  - `ci.yml`: `actions/checkout@v4 → v5`, `pnpm/action-setup@v4 → v5`, `actions/setup-node@v4 → v5`.
  - `release.yml`: `actions/checkout@v4 → v5` (×2), `actions/upload-artifact@v4 → v6`,
    `actions/download-artifact@v4 → v7`, `softprops/action-gh-release@v2 → v3`,
    `docker/login-action@v3 → v4`, `docker/build-push-action@v6 → v7`.
  - `release-prepare.yml`: `actions/checkout@v4 → v5`.
  - Las actions *composite* (`taiki-e/install-action`, `swatinem/rust-cache`,
    `dtolnay/rust-toolchain`) no declaran runtime Node: **no se tocan**.
- **Node unificado en la LTS 24**, con fuente única de verdad:
  - `frontend/package.json`: nuevo `engines.node` (`^24`).
  - `ci.yml`: `node-version: 23` → `node-version-file: frontend/package.json`.
  - `Dockerfile`: `node:23-alpine` → `node:24-alpine`.
- **Guard de consistencia** en `ci.yml`: un paso que falla si el major de Node del `Dockerfile`
  no coincide con el de `engines.node` (así la "fuente única" es verificable, no solo
  documentada).
- **Docs**: `docs/ci-node-24.md` (nuevo) y actualización de `docs/frontend-ci-verification.md`.

## Capabilities

### New Capabilities

- `ci/action-runtimes`: las actions de los workflows se ejecutan sobre un runtime Node soportado
  (24), sin los warnings de deprecación de Node 20.

### Modified Capabilities

- `ci/frontend-verification`: el requisito *Coherencia de toolchain entre CI y build de
  producción* pasa de "Node 23" a "la LTS vigente (24)", leída de una fuente única
  (`engines.node`) y alineada con el `Dockerfile`.

## Impact

- `.github/workflows/ci.yml`, `release.yml`, `release-prepare.yml` — refs de actions y versión de Node.
- `frontend/package.json` — `engines.node` (nuevo).
- `Dockerfile` — tag del stage `frontend-builder`.
- `docs/frontend-ci-verification.md` — decisiones/follow-ups; `docs/ci-node-24.md` (nuevo).
- **Riesgo**: `release.yml`/`release-prepare.yml` no se ejecutan en un PR (solo en tag `v*` o
  push a `main`), así que su validación es `actionlint` + un `workflow_dispatch` de prueba tras
  mergear. `ci.yml` sí se valida en el propio PR.
- **Fuera de alcance**: versión de pnpm (sigue en `packageManager`), lógica de los workflows.
