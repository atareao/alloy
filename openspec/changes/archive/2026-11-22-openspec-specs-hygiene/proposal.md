# Proposal

## Why

`openspec validate --strict --specs` falla en 27 de las 53 specs del repo, así que el comando de verificación de specs no sirve como puerta de CI: cualquier regresión nueva se pierde entre el ruido. Los problemas son de tres tipos:

- **2 errores duros.** `openspec/specs/backend/spec.md` y `openspec/specs/frontend/spec.md` no son specs de capability: son deltas históricos (`# Spec Delta: Backend State Endpoint`, con `## ADDED` / `## REMOVED` / `## MODIFIED`) que se archivaron en la ruta equivocada y quedaron sin `## Purpose` ni `## Requirements`. El validador los rechaza como spec inválida. Aun así documentan comportamiento real y vivo (`GET /api/state` y su contrapartida de frontend).
- **24 warnings de `## Purpose` de relleno.** El `## Purpose` de esas capabilities sigue siendo la frase genérica que escribe `openspec archive` al crearlas. El validador lo detecta como placeholder. Nota clave: un `## Purpose` dentro de un delta **solo se lee al crear** la capability, así que estos hay que corregirlos **editando la spec principal directamente**, no mediante un delta.
- **14 warnings RFC 2119.** El `SHALL`/`MUST` está solo en el header `### Requirement: ...` y no en la primera línea del cuerpo del requirement.

El objetivo es dejar `openspec validate --strict --specs` **en verde** (0 errores, 0 warnings) para que vuelva a ser una puerta útil. Issue #204.

## What Changes

- **Reescribir** `openspec/specs/backend/spec.md` y `openspec/specs/frontend/spec.md` como specs de capability válidas: `# <Capability>` con `## Purpose` real y `## Requirements` (cada uno con `SHALL`/`MUST` en el cuerpo y al menos un `#### Scenario:`). Se preserva el comportamiento hoy documentado (`GET /api/state` con long-poll vía `state_notify`, envelope `StateResponse`, `compute_summary`, y su reflejo en frontend). Se renombran a capabilities con nombre propio (`backend/state-endpoint`, `frontend/state-endpoint`) para no dejar un `spec.md` huérfano en la raíz de `specs/`.
- **Sustituir los 24 `## Purpose` de relleno** por un Purpose real, conciso y específico de cada capability, editando directamente cada `openspec/specs/**/spec.md`.
- **Mover el `SHALL`/`MUST` al cuerpo** de los 14 requirements afectados, dejando el header como descripción y la primera línea del cuerpo con la obligación RFC 2119.
- **Sin cambios de código.** Este change es de documentación/specs: `skip_specs: true` en `.openspec.yaml`. No toca `backend/` ni `frontend/`, no altera comportamiento, y no modifica el contenido normativo de ningún requirement (solo su redacción/formato).

## Capabilities

### New Capabilities

- `backend/state-endpoint`: formaliza como capability la spec huérfana `openspec/specs/backend/spec.md` (endpoint de estado unificado con long-poll).
- `frontend/state-endpoint`: formaliza como capability la spec huérfana `openspec/specs/frontend/spec.md`.

### Modified Capabilities

Las 24 capabilities con `## Purpose` de relleno y las que tienen warnings RFC 2119 (ver `tasks.md` para la lista exacta). En todos los casos la modificación es de redacción (Purpose real / ubicación del `SHALL`), no de contenido normativo.

## Impact

- Solo `openspec/specs/**/*.md` y la creación/renombrado de dos ficheros de capability.
- Sin impacto en código, tests, CI funcional ni despliegue. `openspec validate --strict --specs` pasa a verde.
- Issue: #204.
