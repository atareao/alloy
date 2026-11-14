# Tasks

## 1. RED — Demostrar el aviso

- [x] 1.1 `actionlint .github/workflows/*.yml` **falla** (exit 1) con los 2 `SC2086` de `release.yml`
      (líneas 44 y 47, `>> $GITHUB_ENV`). Guardar la salida literal como evidencia de RED.

## 2. GREEN — Comillar y vigilar

- [x] 2.1 `release.yml`: `>> $GITHUB_ENV` → `>> "$GITHUB_ENV"` en las 2 líneas.
- [x] 2.2 `ci.yml`: job `workflows` (`permissions: contents: read`) que ejecuta
      `rhysd/actionlint:1.7.12` sin argumentos (autodescubre), montaje `:ro` y `--network none`.

## 3. REFACTOR y verificación

- [x] 3.1 `actionlint .github/workflows/*.yml` en verde (exit 0, sin avisos).
- [x] 3.2 YAML válido (`yaml.safe_load`) en `ci.yml` y `release.yml`.
- [x] 3.3 Comprobar la guarda end-to-end: introducir temporalmente un aviso en `release.yml` → el
      comando del job (misma imagen) lo detecta (exit 1); restaurar → limpio.
- [x] 3.4 Comprobar que la imagen incluye `shellcheck` (sin él, `SC####` no se reporta).

## 4. Documentación y cierre

- [x] 4.1 `docs/release-automation.md` (contrato de lint) y retirada del follow-up de
      `docs/ci-node-24.md`.
- [ ] 4.2 Auditar, abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
