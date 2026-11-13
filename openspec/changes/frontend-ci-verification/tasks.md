# Tasks

## 1. Verificación del estado previo (RED)

- [ ] 1.1 Confirmar que el workflow no verifica el frontend: `rg -n "node|pnpm|frontend|tsc" .github/workflows/ci.yml` no devuelve pasos de Node.
- [ ] 1.2 Confirmar que solo hay un job: `rg -n "^  [a-z]+:" .github/workflows/ci.yml` lista solo `test`.
- [ ] 1.3 Capturar baseline verde de CI con la config actual (`gh run list`).

## 2. Job de frontend

- [ ] 2.1 Añadir el job `frontend` a `.github/workflows/ci.yml`, en paralelo al job `test`, sin tocar este último.
- [ ] 2.2 Paso de pnpm: `pnpm/action-setup@v4` **sin** `version` (debe leer `packageManager`).
- [ ] 2.3 Paso de Node: `actions/setup-node@v4` con `node-version: 23` y `cache: pnpm`.
- [ ] 2.4 Paso de instalación: `pnpm install --frozen-lockfile` (debe fallar con lockfile desincronizado).
- [ ] 2.5 Paso de type-check: `pnpm exec tsc -b`.
- [ ] 2.6 Paso de tests: `pnpm run test`.
- [ ] 2.7 Paso de build: `pnpm run build`.
- [ ] 2.8 Paso de lint: `pnpm run lint`.

## 3. Verificación (GREEN)

- [ ] 3.1 YAML válido: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`.
- [ ] 3.2 Reproducir la cadena localmente: `cd frontend && pnpm install --frozen-lockfile && pnpm exec tsc -b && pnpm run test && pnpm run build && pnpm run lint`.
- [ ] 3.3 Empujar la rama y confirmar que **ambos** jobs (`test` y `frontend`) terminan en verde.
- [ ] 3.4 Comprobar que el job falla de verdad: commit temporal con un error de tipos → CI en rojo en el paso de type-check → revertir el commit.
- [ ] 3.5 Comprobar el caso de lockfile desincronizado: modificar `frontend/package.json` sin tocar el lockfile → CI en rojo en el paso de instalación → revertir.
- [ ] 3.6 Confirmar que el job de backend sigue en verde y sin cambios en su definición.

## 4. Cierre

- [ ] 4.1 Documentar el cambio en `docs/frontend-ci-verification.md`.
- [ ] 4.2 Archivar el change: `openspec archive frontend-ci-verification --yes`.
- [ ] 4.3 Verificar que `openspec/changes/frontend-ci-verification/` fue eliminado; si no, `rm -rf`.
- [ ] 4.4 Registrar el follow-up de unificación de la versión de Node (Dockerfile 23 EOL, local 24) con una fuente única de verdad.
