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

El campo `packageManager` fija la versión de pnpm y lo consume `corepack`, de modo que el
entorno de desarrollo y el de build no puedan divergir.

## Configuración

No hay variables de entorno. La versión de pnpm queda fijada en dos sitios que deben
permanecer alineados en su versión mayor:

- `frontend/package.json` → `packageManager: "pnpm@11.21.0"`
- `Dockerfile`, stage `frontend-builder` → `npm install -g pnpm@11.21.0`

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
- **Pin `pnpm@11.21.0`** (D2): replica el pin del `Dockerfile`. El lockfile está validado con
  pnpm 11.x y 12.x, así que no hay incompatibilidad de `lockfileVersion`.
- **Regla muerta fuera** (D3): la exclusión de `package-lock.json` en `.dockerignore` dejó de
  tener sentido al desaparecer el fichero.
- **El requirement npm del spec se retiró con `REMOVED` + `ADDED`** en vez de `MODIFIED`
  porque el validador de OpenSpec no admite renombrar escenarios dentro de un `MODIFIED`.

## Follow-ups

- **`ci.yml` no ejecuta nada del frontend.** Ni `tsc`, ni los 72 tests, ni el build. La
  carencia es previa a este cambio, pero deja el contrato de pnpm sin vigilancia en CI
  (`tasks.md` 5.4). Candidato a un change propio.
- **Verificar Dependabot tras el merge.** El repositorio no tiene `.github/dependabot.yml`
  (la configuración vive en la UI de GitHub): hay que confirmar que el ecosistema `npm` de
  `/frontend` sigue versionando contra `pnpm-lock.yaml`.
- `AGENTS.md` documentaba "18 tests frontend"; la cifra real es **72** (8 ficheros).
