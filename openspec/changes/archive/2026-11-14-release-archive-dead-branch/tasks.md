# Tasks

## 1. RED — Evidenciar el código muerto

- [x] 1.1 Comprobar que la matriz solo declara `archive: tar.gz` (la rama `else` es inalcanzable) y
      que `7z`/`.zip` solo aparecen en esa rama y en referencias del job `release`. Guardar salida.

## 2. GREEN — Eliminar el código muerto

- [x] 2.1 `release.yml`: quitar `archive` de la matriz; *Prepare archive* siempre `tar.gz`.
- [x] 2.2 `release.yml`: quitar `*.zip` de `files:` y `s/\.zip$//` del `sed`.
- [x] 2.3 `docs/release-automation.md`: retirar el follow-up resuelto.

## 3. REFACTOR y verificación

- [x] 3.1 `rg -n '7z|\.zip' .github/workflows/release.yml` sin resultados.
- [x] 3.2 `actionlint .github/workflows/release.yml` sin avisos.
- [x] 3.3 Comprobar que el flujo del release sigue coherente (matriz → `ARTIFACT` → `files:`).

## 4. Documentación y cierre

- [x] 4.1 Auditar, abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
