# Tasks

## 1. Verificación previa (RED)

- [x] 1.1 Confirmar el fallo actual: `gh run view <run> --log-failed` muestra `HTTP 401 Bad credentials` en el paso `Trigger Release workflow`.
- [x] 1.2 Confirmar que el push del tag con `GITHUB_TOKEN` no crea run de `release.yml` (no hay run para `v0.23.10`).
- [x] 1.3 Confirmar que el secreto `GH_PAT` existe: `gh secret list`.

## 2. Arreglo del workflow

- [x] 2.1 `release-prepare.yml`: añadir `token: ${{ secrets.GH_PAT }}` al `actions/checkout`.
- [x] 2.2 Eliminar el paso `Trigger Release workflow` (el `gh workflow run`).
- [x] 2.3 Quitar `actions: write` de `permissions:` (dejar `contents: write`).
- [x] 2.4 Verificar YAML: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-prepare.yml'))"`.

## 3. Verificación (GREEN)

- [x] 3.1 Mergear el arreglo y provocar un cambio en `main` (o `workflow_dispatch` de `release-prepare`) → comprobar que se crea el commit `chore: release vX` y el tag `vX`.
- [x] 3.2 Comprobar que el push del tag **dispara** `release.yml` (aparece el run).
- [x] 3.3 Comprobar que `develop` recibe el commit de release (Sync OK) y `.vampus.yml` coincide con `main`.
- [x] 3.4 Probar `release.yml` con `workflow_dispatch` sobre el tag y verificar `build`/`release`/`docker`.

## 4. Documentación y cierre

- [x] 4.1 `docs/release-automation.md` (flujo, requisito `GH_PAT`, renovación, prueba manual).
- [x] 4.2 Archivar: `openspec archive ci-release-automation --yes`; verificar limpieza del change dir.
