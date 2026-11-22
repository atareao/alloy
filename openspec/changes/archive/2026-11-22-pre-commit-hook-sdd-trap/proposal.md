---
# Proposal

## Why

El hook `pre-commit` (`.githooks/pre-commit`) rechaza cualquier commit que incluya ficheros `*.rs`, `*.ts`, `*.tsx`, `*.js` o `*.jsx` cuando no existe ningún `openspec/changes/*/proposal.md` activo. Esa regla es correcta como puerta del workflow SDD, pero tiene una consecuencia operativa que hoy no está escrita en ningún sitio: si se archiva el change (`openspec archive <feature> --yes`) **antes** de commitear, el directorio del change desaparece de `openspec/changes/` y el commit de esos ficheros fuente queda bloqueado sin proposal al que apuntar.

La salida natural del agente/desarrollador ante ese bloqueo es `git commit --no-verify`, que abre un agujero en la única puerta que separa código sin spec de la rama. El coste de no documentarlo es recurrente: el error aparece cada vez que alguien cierra un change con código pendiente de commit, y el atajo aprendido es el `--no-verify`.

Este change es **solo documentación**: describe la trampa y el orden correcto en `docs/pre-commit-hook.md`. Issue #206.

## What Changes

- Añadir a `docs/pre-commit-hook.md` una sección que documente la trampa del orden `commit → archive`, con:
  - qué extensiones disparan el bloqueo y qué cuenta como "proposal activo" (existencia de `openspec/changes/*/proposal.md`, no el `.approved`);
  - el síntoma exacto del caso "archivé antes de commitear": `COMMIT RECHAZADO: Cambios en código fuente sin change proposal activo`, y su causa;
  - el remedio correcto (commitear el código **antes** de archivar; si ya se archivó, crear un change incremental nuevo, no reabrir el archivado) y por qué `--no-verify` es inaceptable como hábito;
  - la nota relacionada: el hook solo mira lo **staged**, no el working tree.
- Ajustar el `## Follow-ups` del mismo documento si queda incoherente con lo añadido.
- **Sin cambios de código ni de comportamiento del hook.** No se toca `.githooks/pre-commit`, ni `openspec/require-proposal.sh`, ni código Rust/TS.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

Ninguna: este change es doc-only (`skip_specs: true`). La regla que se documenta ya está normada en `openspec/specs/dev/pre-commit-hook/spec.md`; este change no cambia comportamiento, solo lo hace visible al operador.

## Impact

- Documentación: `docs/pre-commit-hook.md`.
- Sin impacto en `backend/`, `frontend/`, tests ni despliegue.
- Riesgo: nulo funcional; riesgo de desactualización si el hook cambia sus extensiones o su lógica de proposal — mitigado citando el fichero canónico como fuente de verdad.