# PLAN-012 — Deploy de co1 reproducible (pin de versión + doc)

- **Issue**: #156
- **Change OpenSpec**: `pin-deploy-co1-version`
- **Rama**: `feature/pin-deploy-co1-version` (desde `develop`)

## Objetivo

Hacer reproducible el deploy de Alloy en co1 pinchando la imagen a una versión explícita (`:0.26.0`) y corrigiendo `docs/deploy-co1.md`, que afirma erróneamente que las releases no publican tag por versión.

## Alcance

- Docs: `docs/deploy-co1.md`.
- Nuevo: `deploy/co1.compose.example.yaml`.
- Ops manual (fuera del repo): actualizar el stack de co1.

## Tareas

Ver `openspec/changes/pin-deploy-co1-version/tasks.md`.

## Criterios de aceptación

- [x] `docs/deploy-co1.md` sin la afirmación falsa y con procedimiento de tag pinchado.
- [x] Ejemplo versionado del compose con imagen pinchada.
- [x] Verificación por tag documentada.
- [ ] Stack de co1 corriendo `:0.26.0` (ops manual, pendiente).
