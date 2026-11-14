# Tasks

## 1. RED — Demostrar la guarda

- [x] 1.1 Con el lock **desincronizado** (versión raíz distinta de `Cargo.toml`), comprobar que
      `cargo test --manifest-path backend/Cargo.toml --locked --no-run` (o
      `cargo metadata --locked`) **falla** con "the lock file ... needs to be updated". Anotar la
      salida literal como evidencia de RED.

## 2. GREEN — Sincronizar y vigilar

- [x] 2.1 `cd backend && cargo update -w` → `backend/Cargo.lock` con `alloy` = `0.24.1` y **una sola
      línea** modificada (`git diff --stat` = 1 insertion / 1 deletion).
- [x] 2.2 `ci.yml`: `--locked` en `cargo test`, en el listado `-- --ignored --list` y en `cargo clippy`
      del job `test`.
- [x] 2.3 `release-prepare.yml`: paso posterior al bump que ejecuta
      `cargo update -w --manifest-path backend/Cargo.toml`.
- [x] 2.4 `.justfile`: `check` con `cargo clippy --locked`; `upgrade` con `cargo update -w`.

## 3. REFACTOR y verificación

- [x] 3.1 `cargo metadata --manifest-path backend/Cargo.toml --locked` y
      `cargo test --manifest-path backend/Cargo.toml --locked` en verde.
- [x] 3.2 Comprobar la guarda end-to-end: bumpear temporalmente la versión de `backend/Cargo.toml`
      (sin tocar el lock) y confirmar que `--locked` falla; restaurar y confirmar que pasa.
- [x] 3.3 `just check` en verde.
- [x] 3.4 Confirmar que `git diff backend/Cargo.lock` solo cambia la línea de versión raíz.

## 4. Documentación y cierre

- [x] 4.1 `docs/cargo-lock-sync.md`: drift, origen, `cargo update -w`, `--locked`, paso de release,
      cómo reproducir/verificar.
- [x] 4.2 Incorporar la auditoría (`@rust-reviewer`): `--locked` también en `release.yml`, `lint` y
      `gf-publish`; delta `MODIFIED` en `ci/release-automation`; requisito del guard local; comandos de
      verificación autocontenidos en la doc.
- [ ] 4.3 Abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
