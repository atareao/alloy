# Proposal

## Why

`frontend/` mantiene hoy **dos lockfiles**: `package-lock.json` (npm) y `pnpm-lock.yaml`.
Pero todo el pipeline real ya asume pnpm: el `Dockerfile` instala con
`pnpm install --frozen-lockfile` y construye con `pnpm run build`, el `.dockerignore`
excluye el lockfile de npm con el comentario *"proyecto usa pnpm"*, y el spec vivo
`build/docker-context` describe escenarios exclusivamente en pnpm.

El resultado es un limbo: dos fuentes de verdad para las mismas dependencias, drift
silencioso entre ellas, y una tarea de auditoría duplicada. Peor aún, el spec vivo
`security/dependency-hygiene` **fija por contrato** `frontend/package-lock.json` y
`npm audit`, de modo que la contradicción no es solo cosmética: el spec bloquea
explícitamente la unificación. Este cambio resuelve ambas cosas a la vez.

## What Changes

- **Eliminar** `frontend/package-lock.json` (lockfile de npm, residuo de uso local).
- **Declarar pnpm como gestor único**: añadir `"packageManager": "pnpm@11.21.0"` a
  `frontend/package.json`, alineado con el pin del `Dockerfile`.
- **BREAKING** (para contributors): la instalación con npm en `frontend/` deja de estar
  soportada; `npm install` / `npm ci` ya no son válidos. Hay que usar
  `pnpm install` (`--frozen-lockfile` en CI).
- **Limpiar `.dockerignore`**: retirar la regla que excluía `frontend/package-lock.json`,
  que queda muerta al desaparecer el fichero.
- **Actualizar la documentación** que aún invoca npm en el frontend: `README.md`,
  `AGENTS.md` y `docs/security-dependencies.md`.
- **Actualizar el contrato del spec** `security/dependency-hygiene`: la auditoría pasa a
  `pnpm audit` sobre `pnpm-lock.yaml`, y se añade el requirement de gestor único.

## Capabilities

### New Capabilities

Ninguna. El comportamiento de la aplicación no cambia; solo el contrato de higiene de
dependencias del frontend.

### Modified Capabilities

- `security/dependency-hygiene`: el requirement *"Lockfiles sin vulnerabilidades
  parcheables"* deja de contemplar `frontend/package-lock.json` y de auditarse con
  `npm audit`; pasa a auditar el único lockfile del frontend (`pnpm-lock.yaml`) con
  `pnpm audit`. El requirement *"Sin regresión funcional"* sustituye las invocaciones
  npm del escenario de frontend por sus equivalentes pnpm. Se añade el requirement
  *"Gestor de paquetes único en el frontend"*.

## Impact

**Ficheros afectados**
- `frontend/package-lock.json` — eliminado
- `frontend/package.json` — nuevo campo `packageManager`
- `.dockerignore` — regla muerta retirada
- `README.md`, `AGENTS.md`, `docs/security-dependencies.md` — referencias npm → pnpm
- `openspec/specs/security/dependency-hygiene/spec.md` — actualizado vía archive

**Código de aplicación**: sin cambios. No se toca `backend/src/` ni `frontend/src/`.

**Build Docker**: sin cambios de comportamiento. El stage `frontend-builder` ya usaba
pnpm; `frontend/package.json` y `frontend/pnpm-lock.yaml` siguen siendo los únicos
ficheros que copia.

**Dependabot**: el repositorio no tiene `.github/dependabot.yml` (la configuración vive
en la UI de GitHub). Hay que verificar tras el cambio que Dependabot sigue versionando
el ecosistema `npm` de `/frontend` apoyándose en `pnpm-lock.yaml`, que es lo que soporta
para proyectos pnpm. La validación queda como tarea explícita.
