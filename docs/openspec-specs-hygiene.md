# Higiene de specs de openspec

## Qué es

Poner `openspec validate --strict --specs` en verde: 0 errores y 0 warnings, para
que el comando sirva como puerta útil de verificación de specs. Issue #204.

## Problema (línea base)

`openspec validate --strict --specs` fallaba en 27 de 53 specs: 2 errores duros +
38 warnings.

- **2 errores duros**: `openspec/specs/backend/spec.md` y `openspec/specs/frontend/spec.md`
  no eran specs de capability, sino deltas históricos (`# Spec Delta: ...` con
  `## ADDED`/`## REMOVED`/`## MODIFIED`) archivados en la ruta equivocada, sin
  `## Purpose` ni `## Requirements`.
- **24 warnings** de `## Purpose` de relleno (la frase genérica de `openspec archive`).
- **14 warnings RFC 2119** en 5 specs: `SHALL`/`MUST` solo en el header, no en el cuerpo.

## Ficheros implicados

- `openspec/specs/backend/state-endpoint/spec.md` y `openspec/specs/frontend/state-endpoint/spec.md`
  — **nuevas** capabilities (sustituyen a los dos ficheros huérfanos borrados).
- 24 `openspec/specs/**/spec.md` — `## Purpose` reescrito.
- 5 `openspec/specs/**/spec.md` — `SHALL`/`MUST` movido al cuerpo del requirement.
- `openspec/changes/openspec-specs-hygiene/` — change proposal SDD (con `skip_specs: true`).
- `plans/PLAN-030.md` — plan del proyecto.

## Cómo funciona

Es una corrección puramente documental, sin cambios de código ni de comportamiento:

1. Las dos specs huérfanas se reescribieron como capabilities válidas
   (`backend/state-endpoint`, `frontend/state-endpoint`) preservando el
   comportamiento que ya documentaban (`GET /api/state`, long-poll vía `state_notify`,
   envelope `StateResponse`, `compute_summary`, y su consumo en frontend vía `useStatePoll`).
2. Los `## Purpose` de relleno se sustituyeron por un propósito real y específico de
   cada capability, editando **directamente la spec principal** (un `## Purpose`
   dentro de un delta solo se lee al crear la capability, así que no sirve para
   reemplazar el existente).
3. El `SHALL`/`MUST` se movió a la primera línea del cuerpo de cada requirement
   afectado. El header queda como descripción. El validador busca el literal en
   inglés, por lo que las obligaciones redactadas en español (`DEBE`/`NO DEBE`)
   pasaron a `MUST`/`MUST NOT`.

## Cómo verificar

```bash
openspec validate --strict --specs   # Totals: 53 passed, 0 failed
```

## Decisiones

- `skip_specs: true` en `.openspec.yaml`: el change no introduce ni modifica
  comportamiento, solo redacción/estructura de las specs. No aplica delta.
- Los dos ficheros huérfanos se **renombraron** a capabilities con nombre propio
  en lugar de fusionarse con `realtime/stream` o `frontend/sse`, para no perder su
  contenido normativo.

## Follow-ups

- Unificar capabilities solapadas (`sse` vs `realtime/stream`, `frontend/sse` vs
  `frontend/state-endpoint`): solo se auditan, no se fusionan aquí.
- Los `ℹ [INFO]` de requirements >500 caracteres no bloquean `--strict`; no se tocan.
