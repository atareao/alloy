# pre-commit-hook

## Qué es

Hook `pre-commit` de Git que bloquea commits que toquen **código fuente** sin un *change proposal*
activo en `openspec/changes/`, y que avisa (sin bloquear) del número de tareas sin completar de cada
change activo. Es la primera línea del workflow SDD del repo.

## Ficheros implicados

- `.githooks/pre-commit` — copia **canónica y versionada** del hook.
- `.justfile` — receta `install-hooks` (`git config core.hooksPath .githooks`).
- `openspec/require-proposal.sh` — preflight que el agente ejecuta; comparte la lógica de "¿hay
  proposal activo?" pero es independiente.

## Cómo funciona

1. Si hay cambios staged en ficheros `*.rs`/`*.ts`/`*.tsx`/`*.js`/`*.jsx` y **no** existe ningún
   `openspec/changes/*/proposal.md`, el commit se **rechaza** con instrucciones.
2. Si hay proposal activo, recorre los `tasks.md` y **avisa** del número de tareas pendientes
   (informativo: no bloquea el commit).

## Cómo habilitar/desplegar

Instalación (una vez por clon):

```sh
just install-hooks
# equivalente a:
git config core.hooksPath .githooks
```

`core.hooksPath` es configuración **local** del repo (no se commitea), por eso hay que ejecutarlo una
vez por clon. Para desinstalar: `git config --unset core.hooksPath`.

## Decisiones

- **Versionado en `.githooks/`**: `.git/hooks/` no se versiona, así que un bug en el hook no era
  corregible vía PR ni reproducible en otro clon.
- **Aviso no bloqueante**: el conteo de tareas es informativo para no impedir commits intermedios.

## Follow-ups

- Evaluar unificar el hook con `openspec/require-proposal.sh` / `validate-workflow.sh` (hoy duplican
  parte de la lógica).
