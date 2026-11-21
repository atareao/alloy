# Tasks

## Checklist

### CI

- [x] Añadir `Swatinem/rust-cache@v2` con `workspaces: backend` en `ci.yml` (job `test`).
- [x] Añadir `Swatinem/rust-cache@v2` con `workspaces: backend` en `release.yml` (job `build`).
- [x] Añadir `docker/setup-buildx-action@v4` en el job `docker` de `release.yml`.
- [x] Añadir `cache-from: type=gha` y `cache-to: type=gha,mode=max` al build-push.

### Verificación

- [x] Ejecutar `actionlint` sobre `.github/workflows/` sin errores.
- [x] Revisar el diff de los workflows.
- [x] Commit con gitmoji y PR a `develop`.
