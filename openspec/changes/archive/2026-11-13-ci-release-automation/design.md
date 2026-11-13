# Design

## Context

- `release-prepare.yml` corre en `push` a `main` (y `workflow_dispatch`); hace bump con
  `vampus`, changelog con `git-cliff`, commit `chore: release vX`, tag `vX` y push.
- `release.yml` se dispara con `push: tags: v*` (y `workflow_dispatch`) y tiene los jobs
  `build` (matriz), `release` y `docker`.
- GitHub **no crea runs** para eventos originados por el `GITHUB_TOKEN` por defecto; y
  `gh workflow run` con `GITHUB_TOKEN` tampoco puede despachar. De ahí el `secrets.GH_PAT`.
- El secreto `GH_PAT` ya está creado en el repo (clave `gh_pat` en el gestor de secretos).

## Goals / Non-Goals

**Goals:**

- Que un merge a `main` publique versión + tag + Release + imagen de forma automática y fiable.
- Que `develop` quede sincronizado con `main` tras el release.
- Eliminar la causa del 401 y el paso redundante.

**Non-Goals:**

- No se cambia `release.yml` (sus jobs quedan como están; se probarán con `workflow_dispatch`).
- No se migra a reusable workflow (`workflow_call`).
- No se toca `ci.yml` ni el resto de workflows.

## Decisions

### D1 — PAT en el checkout en lugar de dispatch manual

`actions/checkout` con `token: ${{ secrets.GH_PAT }}` hace que el `git push` del tag use el
PAT, y ese push **sí** dispara `release.yml`. Es el mecanismo estándar y deja el workflow sin
hacks.
*Alternativa considerada*: mantener `gh workflow run` con el PAT. Rechazada: es un paso extra
que duplica lo que el push del tag ya hace y fue la fuente del 401.
*Alternativa considerada*: reusable workflow `workflow_call` (sin PAT). Rechazada por ahora:
  reestructura `release.yml` y exige pasar el ref como input; más superficie para poca ganancia.

### D2 — Eliminar el paso `Trigger Release workflow`

Con D1 el push del tag ya dispara el release; el paso `gh workflow run Release --ref vX`
sobra. Al quitarlo desaparece el 401 y el "Sync develop" deja de abortar.

### D3 — Permisos del workflow

`permissions: contents: write` (para empujar commit y tag). Se elimina `actions: write`, que
solo era necesario para el dispatch suprimido.

### D4 — El PAT no necesita scope `workflow`

El commit de release solo modifica `.vampus.yml`, `backend/Cargo.toml`, `CHANGELOG.md` y
`LICENSE` — **no** ficheros de `.github/workflows/`, así que `repo` (o fine-grained con
`Contents: write`) basta. Se recomienda `workflow` por si en el futuro el release tocara
workflows.

### D5 — Documentar la dependencia del secreto

`docs/release-automation.md` recoge: qué es, el flujo, el requisito de `GH_PAT`, cómo
renovarlo y cómo probar `release.yml` con `workflow_dispatch`.

## Risks / Trade-offs

- **Caducidad del PAT**: si expira, el release vuelve a fallar. Mitigación: PAT de larga
  duración + documentación + (a futuro) migrar a GitHub App. El fallo ahora sería un error de
  push explícito, no un 401 en un paso de dispatch.
- **`release.yml` sin probar**: el primer release real es la prueba. Se recomienda lanzar un
  `workflow_dispatch` tras mergear el arreglo.
- **Sincronización develop**: el merge `main → develop` puede tocar la versión; al no haber
  conflictos (comprobado) es seguro.
