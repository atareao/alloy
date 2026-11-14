# Proposal

## Why

El `backend/Cargo.lock` commiteado lleva la versión raíz desincronizada respecto a
`backend/Cargo.toml` (`0.23.8` vs `0.24.1`). Cualquier comando `cargo` local sin `--locked`
reescribe el lock, así que el árbol se ensucia (`M backend/Cargo.lock`) en cuanto se toca algo, y
es fácil commitear un lock a medias. El origen es el flujo de release: `.vampus.yml` solo sustituye
la versión en `backend/Cargo.toml`, de modo que cada release re-introduce el drift porque el lock
nunca se actualiza.

## What Changes

- Sincronizar `backend/Cargo.lock` con `Cargo.toml` mediante `cargo update -w` (actualiza **solo** la
  versión raíz del paquete, sin tocar ninguna dependencia).
- `ci.yml`: pasar `--locked` a los comandos cargo que resuelven dependencias (`cargo test`, el
  listado de tests `--ignored` y `cargo clippy`), de forma que el PR falle si el lock está
  desincronizado.
- `release.yml`: `--locked` en la build del artefacto, para que el binario publicado se construya
  siempre desde el lock commiteado.
- `release-prepare.yml`: tras el bump con vampus, sincronizar el lock (`cargo update -w`) para que el
  release no rompa la guarda de `--locked`.
- `.justfile`: `check` pasa a usar `cargo clippy --locked` (detecta el drift ya en el pre-commit
  local); `lint` y `gf-publish` también, por coherencia; y `upgrade` cambia `cargo update` por
  `cargo update -w` (evita subir dependencias que nadie revisó).
- `docs/cargo-lock-sync.md`.

No hay cambios **BREAKING** ni cambios en código de aplicación: solo lockfile, CI y un `justfile`.

## Capabilities

### New Capabilities

- `ci/rust-lockfile`: contrato del lockfile de Rust — el `backend/Cargo.lock` commiteado MUST estar
  sincronizado con `backend/Cargo.toml`, CI MUST fallar cuando no lo esté, y el flujo de release MUST
  mantenerlos sincronizados. También cubre el invariante de que sincronizar la versión no altera la
  versión de ninguna dependencia.

### Modified Capabilities

- `ci/release-automation`: el requisito de bump con vampus se amplía para exigir que el release
  sincronice `backend/Cargo.lock` tras el bump (`cargo update -w`), ya que `release-prepare.yml` está
  gobernado por esa capability y sin este paso la guarda de `--locked` rompería los PRs posteriores al
  primer release.

## Impact

- `backend/Cargo.lock` (una línea: versión raíz `0.23.8` → `0.24.1`).
- `.github/workflows/ci.yml` (`--locked` en el job `test`).
- `.github/workflows/release.yml` (`--locked` en la build del artefacto).
- `.github/workflows/release-prepare.yml` (paso de sincronización del lock).
- `.justfile` (`check` y `upgrade`).
- `docs/cargo-lock-sync.md` (nuevo).
