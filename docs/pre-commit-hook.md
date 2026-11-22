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
- `openspec/changes/<feature>/proposal.md` — lo que el hook considera un proposal activo.

## Cómo funciona

1. Si hay cambios **staged** (el hook mira `git diff --cached`, nunca el working tree) en ficheros que
   casan `\.(rs|tsx?|jsx?)$` — es decir `*.rs`, `*.ts`, `*.tsx`, `*.js`, `*.jsx` — y **no** existe
   ningún `openspec/changes/*/proposal.md`, el commit se **rechaza** con instrucciones.
2. Si hay proposal activo, recorre los `tasks.md` y **avisa** del número de tareas pendientes
   (informativo: no bloquea el commit).

El hook no valida contenido ni aprobación: basta con que exista `openspec/changes/*/proposal.md`. El
marcador `.approved`, el estado del change y su contenido le son indiferentes.

## Cómo habilitar/desplegar

Instalación (una vez por clon):

```sh
just install-hooks
# equivalente a:
git config core.hooksPath .githooks
```

`core.hooksPath` es configuración **local** del repo (no se commitea), por eso hay que ejecutarlo una
vez por clon. Para desinstalar: `git config --unset core.hooksPath`.

## Trampa: orden commit → archive

El hook y `openspec archive` chocan si se usan en el orden equivocado. Archivar
(`openspec archive <feature> --yes`) **elimina** `openspec/changes/<feature>/`, y con él su
`proposal.md`. Si el código de ese change todavía no está commiteado, el commit posterior se encuentra
sin ningún proposal activo y el hook lo rechaza:

```text
❌ [PRE-COMMIT] COMMIT RECHAZADO: Cambios en código fuente sin change proposal activo.
```

La causa no es un bug del hook: es que el directorio que satisfacía el check ya no existe. El orden
correcto es siempre **commit → archive**:

1. Implementar y verificar el código.
2. Commitear con el change todavía activo (`git add -A && git commit`).
3. Solo entonces archivar: `openspec archive <feature> --yes`.
4. Comprobar que `openspec/changes/<feature>/` desapareció (si el archive no lo limpia, borrarlo a
   mano), para no dejar un proposal fantasma que desactive el check sin querer.

Si ya se archivó antes de commitear, el remedio es **abrir un change incremental nuevo** (por ejemplo
`<feature>-followup`) que cubra esos ficheros, no reabrir el archivado ni editar `openspec/changes/archive/`.
`git commit --no-verify` esquiva el hook y deja código sin spec en la rama: es exactamente el fallo que
la puerta existe para impedir, así que no debe usarse como hábito; solo tendría sentido en un commit
puramente documental que además toque algún `.ts/.js` por accidente, y aun así lo correcto es separar
ese fichero en otro commit.

## Decisiones

- **Versionado en `.githooks/`**: `.git/hooks/` no se versiona, así que un bug en el hook no era
  corregible vía PR ni reproducible en otro clon.
- **Aviso no bloqueante**: el conteo de tareas es informativo para no impedir commits intermedios.
- **Se documenta la trampa, no se relaja el hook**: la regla "código fuente exige proposal activo" es
  la que da sentido al SDD; el problema era de orden operativo, no de la comprobación.

## Follow-ups

- Evaluar unificar el hook con `openspec/require-proposal.sh` / `validate-workflow.sh` (hoy duplican
  parte de la lógica).
- Añadir el orden `commit → archive` al propio mensaje de error del hook, para que el remedio aparezca
  en el momento del fallo y no solo en esta página.