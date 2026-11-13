# Verificación del frontend en CI

## Qué es

Job `frontend` en `.github/workflows/ci.yml` que verifica el frontend en cada PR dirigido a
`main` o `develop`. Hasta este cambio, `ci.yml` solo validaba el backend: un error de tipos, un
test roto o un import que rompiera el build de Vite llegaban a `develop` sin que nada lo
detectara.

Change de OpenSpec: `frontend-ci-verification` (archivado en
`openspec/specs/ci/frontend-verification/`).

## Ficheros implicados

- `.github/workflows/ci.yml` — job `frontend` añadido, en paralelo al job `test` del backend
  (que queda intacto).

Nada más. No se toca `backend/`, `frontend/src/`, el `Dockerfile` ni los workflows de release.

## Cómo funciona

El job reproduce, paso a paso, lo que ya se ejecutaba a mano antes de cada PR:

| Paso | Comando |
|---|---|
| Instalar dependencias | `pnpm install --frozen-lockfile` |
| Tipos | `pnpm exec tsc -b` |
| Tests | `pnpm run test` (vitest run) |
| Build | `pnpm run build` (vite) |
| Lint | `pnpm run lint` (oxlint) |

Cada comprobación va en su propio paso para que el fallo sea atribuible: un `failure` dice si el
problema son los tipos o el bundler, en lugar de un único "build failed".

Los dos jobs corren en paralelo, así que el tiempo total no se suma.

## Configuración

No hay variables de entorno. Dos valores que deben permanecer alineados con el build de
producción:

| Qué | Dónde | Valor |
|---|---|---|
| Versión de pnpm | `frontend/package.json` → `packageManager` | `pnpm@11.21.0` |
| Versión de Node | `frontend/package.json` → `engines.node` | `^24` (= `node:24-alpine` del `Dockerfile`) |

El workflow **no** declara la versión de pnpm: `pnpm/action-setup@v5` la resuelve desde
`packageManager` (con `package_json_file: frontend/package.json`, porque el manifest no está en la
raíz). Quien bumpee el gestor solo tiene que tocar `frontend/package.json`.

Tampoco declara la versión de Node: `actions/setup-node@v5` la lee con
`node-version-file: frontend/package.json` (`engines.node`). El tag del `Dockerfile` sigue siendo
una declaración aparte, y un guard en CI (`Check Node version consistency`) falla si divergen.

## Cómo habilitar / desplegar

Nada que habilitar. El workflow se dispara solo en `pull_request` (evento `opened`,
`synchronize`, `reopened`).

Para reproducir la cadena en local:

```bash
cd frontend && pnpm install --frozen-lockfile \
  && pnpm exec tsc -b \
  && pnpm run test \
  && pnpm run build \
  && pnpm run lint
```

## Decisiones

- **Job separado en paralelo** (`design.md` D1): fallos atribuibles y sin sumar tiempo.
- **`pnpm/action-setup@v4` sin `version`** (D2): el pin vive solo en `packageManager`, que es
  justo el campo que introdujo `pnpm-single-package-manager`. Declarar la versión en el workflow
  la duplicaría.
- **Node 24** (D3, revisado en `ci-node-24`): LTS vigente, con `engines.node` como fuente única
  y un guard que la mantiene alineada con el `Dockerfile`.
- **Sin `pnpm audit`**: depende de la base de avisos en red; bloquearía PRs por un advisory nuevo
  ajeno al cambio. La higiene de dependencias la cubren Dependabot y
  `security/dependency-hygiene`.
- **Sin filtros `paths:`** (D4): se prefiere un check determinista y siempre presente a ahorrar
  segundos a costa de que un PR se salte la verificación.
- **Un paso → una comprobación** (D5), aunque `pnpm run build` ya encadene `tsc -b && vite build`.

## Follow-ups

- ~~**Las actions apuntan a Node 20, deprecado.**~~ **Resuelto** en `ci-node-24`: actions a las
  versiones con Node 24.
- ~~**Node 23 está EOL** ... unificar en la LTS vigente con una fuente única de verdad.~~
  **Resuelto** en `ci-node-24`: `engines.node: ^24` como fuente única, `node-version-file` en CI,
  `node:24-alpine` en el `Dockerfile` y guard de consistencia. Ver `docs/ci-node-24.md`.
- **`release-prepare.yml` falla en `main`** (`gh run list` lo muestra en rojo). No lo introduce
  este cambio y no se ha investigado.
- **Tests de integración de `containers`**: dependen de un socket Podman y de contenedores
  concretos (`alloy`, `oxinbox`). Se auto-omiten cuando no hay socket —por eso pasan en CI y en
  un portátil limpio—, pero **fallan en cualquier máquina que sí tenga Podman corriendo sin ese
  contenedor**. Candidato a endurecerlos (omitirse también cuando falte el contenedor concreto).
