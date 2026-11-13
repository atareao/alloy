# Proposal

## Why

La automatización de release está rota desde sept-2026: `release-prepare.yml` hace el bump
(vampus) y el tag, pero **no consigue disparar `release.yml`**. Empuja el tag con el
`GITHUB_TOKEN` por defecto — y GitHub **no crea runs para eventos originados por
`GITHUB_TOKEN`** — así que el paso de rescate `gh workflow run Release` es el único camino,
y falla con **HTTP 401 Bad credentials** porque `secrets.GH_PAT` estaba ausente/expirado.
Al fallar ese paso, además, se aborta el "Sync develop with main" y `develop` queda con la
versión desincronizada (0.23.8 vs 0.23.10 en `main`).

## What Changes

- `release-prepare.yml`: el `actions/checkout` usa `token: ${{ secrets.GH_PAT }}` → el push
  del tag **sí** dispara `release.yml` de forma natural.
- Se **elimina** el paso `Trigger Release workflow` (`gh workflow run`), causa del 401.
- Se elimina el permiso `actions: write` (ya innecesario) y se mantiene `contents: write`.
- Se documenta el requisito del secreto `GH_PAT` y su renovación.
- Sin cambios en la lógica de bump (vampus) ni de changelog (git-cliff).

## Capabilities

### New Capabilities

- `ci/release-automation`: la publicación de una versión (bump + tag + GitHub Release +
  imagen Docker) se dispara de forma fiable al llegar cambios a `main`, y `develop`
  permanece sincronizado.

### Modified Capabilities

Ninguna.

## Impact

- `.github/workflows/release-prepare.yml` — checkout con PAT; paso de dispatch eliminado.
- Secreto requerido: **`GH_PAT`** (ya creado) con scope `repo` (y `workflow` recomendado).
- `release.yml` **no se toca** (queda pendiente probarlo con un `workflow_dispatch`).
- `docs/release-automation.md` (nuevo).
