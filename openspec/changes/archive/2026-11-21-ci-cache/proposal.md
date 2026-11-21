# Proposal

## Why

Los workflows de CI y release recompilan Rust desde cero en cada ejecución. `ci.yml` no cachea el target de Cargo y `release.yml` tampoco, mientras que `release-prepare.yml` ya usa `Swatinem/rust-cache`. Además, el build de la imagen Docker en `release.yml` no usa buildx ni cache de capas, por lo que cada release reconstruye todo.

## What Changes

- Añadir `Swatinem/rust-cache@v2` en `ci.yml` (job `test`) y en `release.yml` (job `build`), con `workspaces: backend`.
- Añadir `docker/setup-buildx-action@v4` en el job `docker` de `release.yml`.
- Añadir `cache-from: type=gha` y `cache-to: type=gha,mode=max` al `docker/build-push-action@v7`.
- Mantener el resto de pasos y versiones de acciones sin cambios.

## Capabilities

### New Capabilities

- `ci/cache`: cache de compilación Rust y de capas Docker en CI/release.

### Modified Capabilities

Ninguna.

## Impact

- Solo `.github/workflows/ci.yml` y `.github/workflows/release.yml`.
- Sin cambios de código, tests ni API.
- Issue: #161.
