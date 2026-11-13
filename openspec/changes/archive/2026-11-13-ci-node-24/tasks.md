# Tasks

## 1. Verificación previa (RED)

- [x] 1.1 Confirmar el warning actual de Node 20 en un run reciente:
      `gh run view <run> --log-failed` / `gh run view <run> --log | rg "Node.js 20 is deprecated"`.
- [x] 1.2 Confirmar las versiones en uso: `rg -n "uses:|node-version|node:2" .github/workflows Dockerfile`.
- [x] 1.3 Baseline de `actionlint` sobre los 3 workflows (debe estar limpio antes y después).

## 2. Actions a Node 24

- [x] 2.1 `ci.yml`: `actions/checkout@v4 → v5`, `pnpm/action-setup@v4 → v5`, `actions/setup-node@v4 → v5`.
- [x] 2.2 `release.yml`: `actions/checkout@v4 → v5` (build y docker), `actions/upload-artifact@v4 → v6`,
      `actions/download-artifact@v4 → v7`, `softprops/action-gh-release@v2 → v3`,
      `docker/login-action@v3 → v4`, `docker/build-push-action@v6 → v7`.
- [x] 2.3 `release-prepare.yml`: `actions/checkout@v4 → v5` (dejar `taiki-e/install-action` y
      `swatinem/rust-cache` como están).

## 3. Node unificado en la LTS 24

- [x] 3.1 `frontend/package.json`: añadir `"engines": { "node": "^24" }`.
- [x] 3.2 `ci.yml`: sustituir `node-version: 23` por `node-version-file: frontend/package.json`.
- [x] 3.3 `Dockerfile`: `node:23-alpine` → `node:24-alpine` en el stage `frontend-builder`.
- [x] 3.4 `ci.yml`: añadir el paso *guard* que falla si el major de `engines.node` ≠ major del
      `Dockerfile`.

## 4. Verificación (GREEN)

- [x] 4.1 YAML válido: `python3 -c "import yaml,glob;[yaml.safe_load(open(f)) for f in glob.glob('.github/workflows/*.yml')]"`.
- [x] 4.2 `actionlint` sin issues **nuevos** en los 3 workflows (quedan 2 `SC2086` info
      preexistentes en `release.yml`, ajenos al cambio).
- [x] 4.3 Simular el guard en local (extraer ambos majors y compararlos).
- [x] 4.4 PR: el job `frontend` (con setup-node desde `engines.node`) y el job `test` en verde.
- [ ] 4.5 **(post-merge)** Verificar el pipeline de release con las actions nuevas en el
      release real `develop→main` (v0.24.1): workflows *Prepare Release* y *Release* en verde y
      sin warnings de Node 20.

## 5. Documentación y cierre

- [x] 5.1 `docs/ci-node-24.md` (qué es, ficheros, cómo funciona, decisiones, verificación).
- [x] 5.2 Actualizar `docs/frontend-ci-verification.md` (decisión Node 23 → 24 y follow-ups
      resueltos).
- [ ] 5.3 Archivar: `openspec archive ci-node-24 --yes`; verificar limpieza del change dir.
