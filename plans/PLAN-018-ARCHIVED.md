# PLAN-018 — Cache de CI y release (#161)

**Issue:** #161
**Change OpenSpec:** `openspec/changes/ci-cache/`
**Rama:** `feature/ci-cache` (desde `develop`)

## Objetivo

Reducir el tiempo de CI y release cacheando la compilación Rust y las capas Docker, sin cambiar tests ni comportamiento funcional.

## Mapa de cambios

- `.github/workflows/ci.yml`: `Swatinem/rust-cache@v2` en el job `test`.
- `.github/workflows/release.yml`: `Swatinem/rust-cache@v2` en el job `build`; `docker/setup-buildx-action@v4` y cache GHA en el job `docker`.

## Tareas

1. Añadir rust-cache en `ci.yml`.
2. Añadir rust-cache en `release.yml`.
3. Añadir setup-buildx y cache GHA en el build Docker.
4. Validar con `actionlint`.
5. Revisar diff, commit y PR.

## Criterios de aceptación

- [x] `ci.yml` usa `Swatinem/rust-cache@v2` con `workspaces: backend`.
- [x] `release.yml` usa `Swatinem/rust-cache@v2` con `workspaces: backend`.
- [x] `release.yml` usa `docker/setup-buildx-action@v4`.
- [x] El build Docker usa `cache-from: type=gha` y `cache-to: type=gha,mode=max`.
- [x] `actionlint` pasa sin errores.
- [x] Diff limitado a los dos workflows + artefactos OpenSpec/plan.
