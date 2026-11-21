# Proposal

## Why

`docs/deploy-co1.md` afirma que las releases publican `:latest` «(no tag por versión)». Es **falso**: `.github/workflows/release.yml:98` publica `ghcr.io/atareao/alloy:${VERSION}` **y** `:latest`, y GHCR ya tiene tags por versión (p. ej. `0.26.0`). El deploy de co1 con `:latest` es **no reproducible** (no se sabe qué versión corre) y la doc induce a error (#156).

## What Changes

- Corregir `docs/deploy-co1.md`: eliminar la afirmación falsa; documentar que las releases publican `:X.Y.Z` además de `:latest`.
- Cambiar el procedimiento para **pinchar la versión explícita** (`:0.26.0`) en el `compose.yaml` del stack de co1 y **verificar por tag** (no por digest/fecha).
- Añadir al repo un **ejemplo versionado** del compose del stack (`deploy/co1.compose.example.yaml`) con la imagen pinchada a una versión, para que el procedimiento sea auditable y reproducible.
- Documentar el paso de **ops manual** de actualizar el stack de co1 a la versión pinchada.

## Capabilities

### New Capabilities

- `dev/deploy-co1`: contrato del (re)despliegue de Alloy en co1 — imagen pinchada a una versión explícita (nunca `:latest` como referencia desplegada), verificación por tag, y documentación coherente con los artefactos reales del release.

### Modified Capabilities

- Ninguna.

## Impact

- **Docs**: `docs/deploy-co1.md` (corrección + procedimiento).
- **Nuevo**: `deploy/co1.compose.example.yaml` (ejemplo versionado con imagen pinchada).
- **Ops (manual, fuera del repo)**: actualizar `/home/lorenzo/docker/dockge/stacks/alloy/compose.yaml` en co1 a `:0.26.0` y redeploy.
- **Código**: sin cambios en backend/frontend. Sin migración. Sin dependencias nuevas.
