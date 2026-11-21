# Tasks — pin-deploy-co1-version

## 1. Documentación y procedimiento

- [x] 1.1 `docs/deploy-co1.md`: eliminar la afirmación falsa «no tag por versión»; documentar `:X.Y.Z` + `:latest`.
- [x] 1.2 `docs/deploy-co1.md`: procedimiento con imagen pinchada (`:0.26.0`) y verificación **por tag**.
- [x] 1.3 Crear `deploy/co1.compose.example.yaml`: ejemplo versionado del stack con la imagen pinchada.

## 2. Verificación (docs — no TDD)

- [x] 2.1 `rg` para confirmar que no queda ninguna afirmación «no tag por versión» en `docs/`.
- [x] 2.2 Validar el YAML del ejemplo (`docker compose -f deploy/co1.compose.example.yaml config` o lint YAML).

## 3. Ops (manual, fuera del repo)

- [ ] 3.1 En co1: editar `compose.yaml` a `ghcr.io/atareao/alloy:0.26.0` y `docker compose up -d`.
- [ ] 3.2 Verificar tag en ejecución (`docker inspect`) y HTTP 200 en `https://alloy.territoriolinux.es/`.

## 4. Docs de la feature y archivo

- [x] 4.1 `docs/pin-deploy-co1-version.md`.
- [ ] 4.2 `openspec archive pin-deploy-co1-version` tras merge (PR a `develop`).
