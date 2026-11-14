# Proposal

## Why

`actionlint` reporta **2 avisos** `SC2086:info` (shellcheck) en `.github/workflows/release.yml`,
paso *Prepare archive*, en las dos escrituras a `$GITHUB_ENV` (`>> $GITHUB_ENV`, sin comillas). No
rompen el release hoy (la ruta no tiene espacios), pero son ruido de lint y el repo no ejecuta
`actionlint` en CI, así que cualquier aviso futuro pasaría desapercibido.

## What Changes

- `release.yml`: comillar `"$GITHUB_ENV"` en las 2 escrituras (comportamiento idéntico).
- `ci.yml`: nuevo job `workflows` (`permissions: contents: read`) que ejecuta `actionlint` con la imagen
  fijada `rhysd/actionlint:1.7.12` (que incluye `shellcheck`, imprescindible para los diagnósticos
  `SC####`), de modo que un workflow que no pase `actionlint` haga fallar el PR.
- `docs/release-automation.md`: nota del contrato de lint; retirada del follow-up ya resuelto de
  `docs/ci-node-24.md`.

Sin cambios de comportamiento del release ni de código de aplicación.

## Capabilities

### New Capabilities

- `ci/workflow-lint`: contrato del lint de workflows — los scripts embebidos (`run:`) MUST pasar
  `actionlint` sin avisos (con las escrituras a `$GITHUB_ENV`/`$GITHUB_OUTPUT` comilladas) y CI MUST
  ejecutarlo y fallar ante cualquier aviso, con `shellcheck` disponible.

### Modified Capabilities

- Ninguna. `ci/release-automation` no se modifica: el cambio solo comilla dos líneas de `release.yml`
  y añade el guard, sin alterar su comportamiento especificado.

## Impact

- `.github/workflows/release.yml` (2 líneas: `$GITHUB_ENV` → `"$GITHUB_ENV"`).
- `.github/workflows/ci.yml` (nuevo job `workflows`).
- `docs/release-automation.md`, `docs/ci-node-24.md`.
