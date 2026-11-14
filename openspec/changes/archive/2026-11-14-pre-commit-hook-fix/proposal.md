# Proposal

## Why

El hook `pre-commit` del repo tiene un bug y no está versionado:

1. **Bug del contador de tareas**: `INCOMPLETE=$(grep -c '\[ \]' … || echo 0)`. `grep -c` imprime
   `0` y sale con código 1 cuando no hay coincidencias, así que con 0 tareas incompletas la variable
   queda `"0\n0"` y `[ "$INCOMPLETE" -gt 0 ]` falla con *"se esperaba un entero"*. Reproducido.
2. **No versionado**: el hook solo existe en `.git/hooks/pre-commit` (por máquina, sin seguimiento
   en git). No hay copia canónica ni receta de instalación, así que el bug no se puede corregir vía
   PR ni reproducir en otro clon.

## What Changes

- Nuevo `.githooks/pre-commit` (copia canónica del hook, con el contador corregido:
  `grep -c … || true` + `${VAR:-0}`).
- `.justfile`: receta `install-hooks` que configura `core.hooksPath=.githooks` y avisa.
- `docs/pre-commit-hook.md`: qué hace, cómo instalarlo y la corrección.
- `README.md`: mención en la sección de desarrollo.

Sin cambios en código de aplicación, CI ni en el contrato de `openspec/require-proposal.sh`.

## Capabilities

### New Capabilities

- `dev/pre-commit-hook`: contrato del hook que bloquea commits de código fuente sin change proposal
  activo y avisa de tareas incompletas, instalable y reproducible desde el repo.

### Modified Capabilities

- Ninguna.

## Impact

- `.githooks/pre-commit` (nuevo), `.justfile` (`install-hooks`), `docs/pre-commit-hook.md` (nuevo),
  `README.md`.
- Requiere que cada clon ejecute `just install-hooks` (o `git config core.hooksPath .githooks`) una vez.
