# Gestor de paquetes único en el frontend (pnpm)

## Qué es

Unificación del gestor de paquetes del frontend en **pnpm**, eliminando el lockfile de npm
(`frontend/package-lock.json`) que convivía con `frontend/pnpm-lock.yaml`. Sin cambios de
código de aplicación ni de comportamiento: mismas dependencias, mismos rangos, mismo build.

Change de OpenSpec: `openspec/changes/pnpm-single-package-manager/` (archivado en
`openspec/specs/security/dependency-hygiene/`).

## Ficheros implicados

- `frontend/package-lock.json` — **eliminado**.
- `frontend/pnpm-lock.yaml` — único lockfile del frontend (sin cambios).
- `frontend/package.json` — nuevo campo `"packageManager": "pnpm@11.21.0"`.
- `.dockerignore` — retirada la regla que excluía `frontend/package-lock.json`.
- `README.md`, `AGENTS.md`, `docs/security-dependencies.md` — referencias npm → pnpm.
- `openspec/specs/security/dependency-hygiene/spec.md` — contrato actualizado.

## Cómo funciona

pnpm ya era el gestor real del proyecto antes de este cambio: el `Dockerfile` instala y
construye con pnpm desde el stage `frontend-builder`. Lo que faltaba era eliminar la
ambigüedad del lockfile de npm que se usaba de forma ocasional en local.

El campo `packageManager` fija la versión de pnpm en un único sitio. Lo consumen el `Dockerfile`
(que la deriva con `node -p`), el workflow de CI (`pnpm/action-setup`, sin `version`) y `corepack`
en local, de modo que el entorno de desarrollo y el de build no puedan divergir.

## Configuración

No hay variables de entorno. La versión de pnpm vive en un **único sitio**:

- `frontend/package.json` → `packageManager: "pnpm@11.21.0"`

Tanto el `Dockerfile` (stage `frontend-builder`, que la deriva con `node -p`) como el workflow de CI
(`pnpm/action-setup`, sin `version`) la leen de ahí: ninguna otra parte declara una versión de pnpm,
así que bumpear el gestor es un cambio de una sola línea.

## Cómo habilitar / desplegar

Nada que habilitar. Para desarrollo e integración continua:

```bash
cd frontend && pnpm install --frozen-lockfile   # instalación reproducible
cd frontend && pnpm run dev                     # servidor Vite (hot reload)
cd frontend && pnpm run test                    # 72 tests
cd frontend && pnpm exec tsc -b && pnpm run build
cd frontend && pnpm audit                       # higiene de dependencias
```

## Decisiones

- **pnpm como gestor canónico** (`design.md` D1): ya era el del build de producción. Elegir
  npm habría implicado reescribir el `Dockerfile`, el `.dockerignore` y el spec
  `build/docker-context` para no ganar nada.
- **Pin `pnpm@11.21.0`** (D2): única fuente de verdad en `packageManager`. El lockfile está
  validado con pnpm 11.x y 12.x, así que no hay incompatibilidad de `lockfileVersion`. El
  `Dockerfile` y el workflow de CI la derivan de ahí (change `dockerfile-pnpm-single-source`).
- **Regla muerta fuera** (D3): la exclusión de `package-lock.json` en `.dockerignore` dejó de
  tener sentido al desaparecer el fichero.
- **El requirement npm del spec se retiró con `REMOVED` + `ADDED`** en vez de `MODIFIED`
  porque el validador de OpenSpec no admite renombrar escenarios dentro de un `MODIFIED`.

## Follow-ups

- **`ci.yml` no ejecutaba nada del frontend** — resuelto por el change `frontend-ci-verification`
  (job `frontend` con type-check, tests, build y lint, y guards de versión de Node y pnpm).
- **Verificar Dependabot tras el merge.** El repositorio no tiene `.github/dependabot.yml`
  (la configuración vive en la UI de GitHub): hay que confirmar que el ecosistema `npm` de
  `/frontend` sigue versionando contra `pnpm-lock.yaml`.
- `AGENTS.md` documentaba "18 tests frontend"; la cifra real es **72** (8 ficheros).
