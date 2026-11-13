# ci-node-24

## Qué es

Puesta al día del toolchain de CI/release (issue
[#99](https://github.com/atareao/alloy/issues/99)):

1. **Actions a Node 24.** Todas las actions basadas en Node de los tres workflows usan ya la
   **primera versión mayor cuyo `action.yml` declara `runs.using: node24`**, eliminando los
   warnings *"Node.js 20 is deprecated"* que aparecían en cada ejecución.
2. **Node unificado en la LTS 24.** `Dockerfile`, `ci.yml` y `frontend/package.json` dejan de
   declarar versiones distintas (23 en el `Dockerfile`/CI, 24 en local) y pasan a la LTS vigente
   con una **fuente única de verdad**.
3. **Guard de consistencia** en CI que impide volver a divergir.

## Ficheros implicados

- `.github/workflows/ci.yml` — actions a v5/v5/v5, `node-version-file`, paso *Check Node version
  consistency*.
- `.github/workflows/release.yml` — actions a v5/v6/v7/v3/v4/v7.
- `.github/workflows/release-prepare.yml` — `actions/checkout@v5`.
- `Dockerfile` — stage `frontend-builder` sobre `node:24-alpine`.
- `frontend/package.json` — `engines.node: "^24"` (fuente única de verdad).

## Cómo funciona

### Fuente única de verdad de la versión de Node

`frontend/package.json` declara `"engines": { "node": "^24" }`. El job `frontend` de `ci.yml`
**no** declara una `node-version` propia: usa `node-version-file: frontend/package.json`, y
`actions/setup-node` lee `engines.node` (la ruta se resuelve relativa a la raíz del repo, por eso
se indica `frontend/package.json` y no `package.json`).

Es el mismo principio que ya se aplicaba a pnpm (`packageManager`): una sola declaración por
herramienta.

### Guard de consistencia

Un `FROM` de Docker **no puede** leer `package.json`, así que el tag del stage `frontend-builder`
es una declaración aparte. Para que la alineación no dependa de la memoria, el job `frontend`
ejecuta un paso previo al setup de Node:

```bash
PKG_MAJOR=$(grep -oE '"node"[[:space:]]*:[[:space:]]*"\^?[0-9]+' frontend/package.json | grep -oE '[0-9]+$')
DOCKER_MAJOR=$(grep -oE '^FROM .*node:[0-9]+' Dockerfile | grep -oE '[0-9]+$')
[ "$PKG_MAJOR" = "$DOCKER_MAJOR" ] || exit 1
```

Si alguien sube el `Dockerfile` a `node:25-alpine` sin tocar `engines.node` (o al revés), el job
falla con `::error::Node desalineado`.

### Versiones de actions

Se elige la **mayor mínima** con Node 24, no la última disponible, porque `release.yml` y
`release-prepare.yml` **no se ejecutan en un PR** y conviene minimizar cambios de comportamiento.

| Action | Antes | Ahora | Runtime |
|---|---|---|---|
| `actions/checkout` | v4 | **v5** | node24 |
| `actions/upload-artifact` | v4 | **v6** | node24 |
| `actions/download-artifact` | v4 | **v7** | node24 |
| `actions/setup-node` | v4 | **v5** | node24 |
| `pnpm/action-setup` | v4 | **v5** | node24 |
| `docker/login-action` | v3 | **v4** | node24 |
| `docker/build-push-action` | v6 | **v7** | node24 |
| `softprops/action-gh-release` | v2 | **v3** | composite |

Las actions **composite** (`taiki-e/install-action`, `swatinem/rust-cache`, `dtolnay/rust-toolchain`)
no declaran runtime Node: se dejan como estaban.

## Configuración

No hay variables de entorno ni secretos nuevos. Para cambiar de LTS en el futuro hay que tocar
**dos** sitios (el guard lo exige):

1. `frontend/package.json` → `engines.node`.
2. `Dockerfile` → tag del stage `frontend-builder`.

## Cómo habilitar / desplegar

- Se aplica al mergear; no requiere despliegue.
- **Verificación en PR**: el job `frontend` de `ci.yml` valida las actions nuevas, la lectura de
  `engines.node` y el guard.
- **Verificación del pipeline de release** (`release.yml`, `release-prepare.yml`): `actionlint` +
  comprobación de que los refs existen (API de GitHub) + el siguiente release real. Un
  `workflow_dispatch` sobre un tag ya existente usaría la versión **antigua** del workflow, así
  que no sirve para validar el cambio.

## Decisiones

- **Mayor mínima con Node 24** (design D1): menos superficie de cambio en un pipeline sin
  cobertura de PR.
- **`engines.node` como fuente única** (D2): mismo principio que `packageManager` para pnpm;
  descartado `.nvmrc` por ser una cuarta ubicación.
- **Guard en CI** (D3/D4): convierte la alineación en algo verificable.
- **Composite intactas** (D5).

## Follow-ups

- Los 2 avisos `SC2086` (info) de `actionlint` en `release.yml` (línea ~37) son **preexistentes**
  y ajenos a este cambio (citado de `$GITHUB_ENV` en el paso *Prepare archive*). Se dejan como
  están para no mezclar.
- El `Dockerfile` fija `pnpm@11.21.0` por duplicado respecto a `packageManager`; candidato a
  alinearse igual que se ha hecho con Node.
- `release.yml`/`release-prepare.yml` quedan sin ejercitar hasta el próximo release real.
