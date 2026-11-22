# PLAN-030 — Higiene de specs de openspec

- **Issue:** #204 — chore(openspec): higiene de specs — Purpose real y RFC 2119
- **Change proposal SDD:** `openspec/changes/openspec-specs-hygiene/`
- **Rama:** `feature/openspec-specs-hygiene` (desde `develop`)
- **Estado:** borrador — pendiente de aprobación explícita del spec antes de tocar specs
- **Fecha:** 2026-11-22

## Objetivo

Dejar `openspec validate --strict --specs` en verde: 0 errores y 0 warnings (hoy: 2 errores + 38 warnings en 27 de 53 specs).

## Alcance

1. **2 errores duros** — `openspec/specs/backend/spec.md` y `openspec/specs/frontend/spec.md` son deltas históricos sin `## Purpose`/`## Requirements`. Se reescriben como capabilities válidas (`backend/state-endpoint`, `frontend/state-endpoint`) preservando el comportamiento de `GET /api/state`.
2. **24 warnings de Purpose de relleno** — sustituir el `## Purpose` genérico por uno real, editando directamente cada `openspec/specs/**/spec.md` (un Purpose en delta no sustituye al existente).
3. **14 warnings RFC 2119** (5 specs) — mover el `SHALL`/`MUST` al cuerpo del requirement.

Sin cambios de código: el change va con `skip_specs: true`.

## Tareas

Ver checklist detallada en `openspec/changes/openspec-specs-hygiene/tasks.md`.

## Verificación

- `openspec validate --strict --specs` → 0 errores, 0 warnings.
- `openspec list --specs` sin pérdida de requirements.
- PR a `develop` (merge commit) + cierre manual del issue #204.
