# release-automation

## Qué es

Automatización de publicación de versiones de Alloy: al integrar cambios en `main`, se
calcula la nueva versión, se genera el changelog, se crea el commit de release y el tag, y el
tag dispara el workflow **Release** (binarios + GitHub Release + imagen Docker). Además,
`develop` se sincroniza con `main`.

## Ficheros implicados

- `.github/workflows/release-prepare.yml` — bump (vampus), changelog (git-cliff), commit, tag,
  push y sincronización de `develop`.
- `.github/workflows/release.yml` — se dispara con `push: tags: v*`; jobs `build`, `release`
  y `docker`.
- `.vampus.yml` — `current_version` y el patrón que reescribe `backend/Cargo.toml`.
- `backend/Cargo.lock` — se sincroniza con el bump; ver `docs/cargo-lock-sync.md`.
- `.justfile` — `version := \`vampus show\``.
- `.github/workflows/ci.yml` — job `workflows`: `actionlint` sobre `.github/workflows/`.

## Cómo funciona

1. Push a `main` (o `workflow_dispatch`) lanza `release-prepare.yml`, salvo que el commit sea
   `chore: release …` (guarda anti-bucle).
2. Calcula el bump desde los commits: `!: ` ⇒ major · `^feat` ⇒ minor · resto ⇒ patch.
3. `vampus upgrade <tipo>` actualiza `.vampus.yml` + `backend/Cargo.toml`; el lock se sincroniza
   (`cargo update -w --manifest-path backend/Cargo.toml`, ver `docs/cargo-lock-sync.md`) y `git-cliff`
   genera `CHANGELOG.md`.
4. Commit `chore: release vX`, tag `vX`, push a `main` y del tag — **con `GH_PAT`**, para que
   el push del tag **sí** dispare `release.yml`.
5. `release.yml` construye binarios, crea la GitHub Release y publica la imagen en GHCR.
6. `develop` se sincroniza con `main` (merge) para no divergir de versión.

## Configuración — secreto `GH_PAT`

El workflow depende del secreto **`GH_PAT`** en `Settings → Secrets and variables → Actions`.
Es imprescindible porque GitHub **no crea runs** para eventos originados por el `GITHUB_TOKEN`
por defecto (por eso el tag empujado con el token por defecto no dispararía el Release).

- Scopes: `repo` (classic) o fine-grained con `Contents: write` (y `Actions: write` /
  `workflow` recomendados por si el release tocara workflows).
- **Renovación**: si el PAT caduca, el release fallará en el `git push`. Regenerarlo en GitHub
  y actualizar el secreto (`gh secret set GH_PAT --repo atareao/alloy`).

## Lint de workflows

Los workflows pasan `actionlint` **sin avisos**. CI ejecuta un job dedicado (`workflows` en
`.github/workflows/ci.yml`) que corre la imagen `rhysd/actionlint:1.7.12` **sin argumentos**
(`actionlint` autodescubre `.github/workflows/`; pasar el directorio como argumento falla). Cualquier
aviso nuevo hace fallar el PR. Antes este lint solo se ejecutaba a mano, así que los avisos pasaban
desapercibidos.

En local, usa la misma imagen (incluye `shellcheck`):

```bash
docker run --rm -v "$PWD:/mnt:ro" -w /mnt rhysd/actionlint:1.7.12
# o, si tienes actionlint Y shellcheck instalados:
actionlint
```

⚠️ `actionlint` **no** detecta los `SC####` sin `shellcheck` en `PATH`: en ese caso sale 0 aunque el
problema esté presente. La imagen de CI lo incluye.

Los scripts embebidos (`run:`) deben comillar las escrituras a los ficheros de entorno de GitHub
(`>> "$GITHUB_ENV"`, `"$GITHUB_OUTPUT"`, `"$GITHUB_PATH"`): sin comillas, `shellcheck` reporta
`SC2086` y el job de CI falla.

## Cómo habilitar / probar

- **Release normal**: mergear a `main`; el workflow hace el resto.
- **Probar sin release**: `gh workflow run Release --ref vX` (o el botón *Run workflow* en la
  pestaña Actions), que usa `workflow_dispatch` de `release.yml`.
- **Verificar**: `gh run list --workflow=release.yml` debe mostrar un run para el tag nuevo.

## Decisiones

- PAT en el `actions/checkout` en vez de `gh workflow run` (design D1/D2): el push del tag
  dispara el Release de forma natural y desaparece el 401.
- `permissions: contents: write` únicamente (D3).
- El PAT no necesita scope `workflow` mientras el release no toque `.github/workflows/` (D4).
- La imagen de `actionlint` en CI se fija a `1.7.12`: resultado reproducible y sin ejecutar el
  script de descarga oficial, que corre código remoto de `main` sin fijar.

## Follow-ups

- Probar `release.yml` (jobs `build`/`release`/`docker`) con un `workflow_dispatch` real.
- Valorar migrar de PAT a **GitHub App** para evitar caducidades.
- `release.yml` no se ha ejecutado desde su reescritura (sept-2026).
