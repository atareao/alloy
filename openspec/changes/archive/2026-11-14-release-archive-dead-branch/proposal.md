# Proposal

## Why

En `.github/workflows/release.yml` el paso *Prepare archive* tiene un `if/else` que elige entre
`tar.gz` y `zip` según `matrix.archive`, pero la matriz solo declara `archive: tar.gz` para sus dos
targets. Por tanto la rama `else` (`7z a "$PKG.zip"`, `ARTIFACT=$PKG.zip`) es **inalcanzable**:
código muerto. Además `7z` no está garantizado en los runners, así que si alguna vez se ejecutara
fallaría. El job `release` arrastra referencias colgantes a ese formato (`*.zip` en `files:` y
`s/\.zip$//` en el `sed`), aunque ningún `.zip` se produce ya.

## What Changes

- `.github/workflows/release.yml`:
  - Se elimina la clave `archive` de la matriz (solo la consumía la rama condicional).
  - *Prepare archive* produce **siempre** `tar.gz` (sin `if/else`, sin `7z`, sin `.zip`).
  - El job `release` deja de referenciar `*.zip` (`files:` y el `sed` del generador de la tabla).
- `docs/release-automation.md`: se retira el follow-up (resuelto).

Sin cambios en código de aplicación ni en el resto de workflows.

## Capabilities

### Modified Capabilities

- `ci/release-automation`: se añade (ADDED) el requisito de que el workflow de Release no contenga
  ramas ni referencias a formatos de artefacto que la matriz no produzca.

## Impact

- `.github/workflows/release.yml`, `docs/release-automation.md`.

