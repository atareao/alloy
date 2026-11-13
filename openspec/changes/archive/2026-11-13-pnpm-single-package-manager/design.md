# Design

## Context

El frontend arrastra dos lockfiles desde hace tiempo: `package-lock.json` (120 KB) y
`pnpm-lock.yaml` (87 KB). El pipeline real ya es pnpm desde antes de este cambio:

- `Dockerfile`, stage `frontend-builder`: `npm install -g pnpm@11.21.0`,
  `COPY frontend/package.json frontend/pnpm-lock.yaml ./`,
  `pnpm install --frozen-lockfile`, `pnpm run build`.
- `.dockerignore`: excluye `frontend/package-lock.json` con el comentario
  *"proyecto usa pnpm"*, y excluye `frontend/node_modules/` para no pisar la instalación
  de la imagen.
- `openspec/specs/build/docker-context/spec.md`: todos sus escenarios hablan de pnpm.
- `docs/security-dependencies.md`: describe el lockfile npm como *"uso local"* y ya listaba
  *"unificar en un único gestor"* como follow-up.

Restricciones relevantes:

- El entorno local tiene `pnpm 12.15.0`; el `Dockerfile` fija `11.21.0`. El lockfile
  actual ha sido validado con **ambas** versiones (`pnpm install --frozen-lockfile`
  completa sin error en las dos), así que no hay incompatibilidad de `lockfileVersion`.
- `openspec/specs/security/dependency-hygiene/spec.md` fija por contrato
  `frontend/package-lock.json` y `npm audit`, por lo que el spec debe modificarse en el
  mismo cambio.
- No existe `.github/dependabot.yml`: la configuración de Dependabot vive en la UI de
  GitHub y no es versionada.

## Goals / Non-Goals

**Goals:**

- Un único lockfile en `frontend/`, con pnpm como gestor declarado de forma machine-readable.
- Eliminar las invocaciones npm de la documentación dirigida a contributors.
- Alinear el spec `security/dependency-hygiene` con el estado real (pnpm-only).
- Cero cambios de comportamiento: mismo `package.json`, mismas dependencias, mismo build.

**Non-Goals:**

- **No** se añade un job de frontend a la CI. Hoy `ci.yml` solo ejecuta `cargo test`,
  `cargo fmt --check` y `clippy`; los 18 tests del frontend no se validan en CI. Es una
  carencia real y previa a este cambio, pero arreglarla es otro alcance.
- **No** se reescribe el `Dockerfile` ni se sube el pin de pnpm.
- **No** se tocan las referencias a npm en `openspec/changes/archive/**` ni en
  `docs/superpowers/plans/**`: son registro histórico y reflejan el estado de su momento.
- **No** se toca código de aplicación (`backend/src/`, `frontend/src/`).

## Decisions

### D1 — El gestor canónico es pnpm, no npm

Se elige pnpm porque **ya es** el gestor del build de producción: el `Dockerfile` instala
y construye con pnpm desde antes de este cambio. Elegir npm habría implicado reescribir el
stage `frontend-builder`, el `.dockerignore` y el spec `build/docker-context`, es decir,
cambiar el camino que hoy funciona, por cero beneficio.

*Alternativa descartada:* npm-only (rehacer el Dockerfile). *Alternativa descartada:*
yarn (introduciría un tercer gestor sin ninguna razón).

### D2 — `packageManager` fijado a `pnpm@11.21.0`

Se añade `"packageManager": "pnpm@11.21.0"` a `frontend/package.json`, replicando
exactamente el pin del `Dockerfile`. La justificación es que el campo lo consume corepack y
su función es hacer que local y build no puedan divergir; fijarlo a otra versión
reintroduce la divergencia que este cambio elimina.

*Alternativa descartada:* fijar `12.15.0` (la versión del entorno de desarrollo de hoy).
Dejaría local y CI en versiones distintas y obligaría a bumpear el `Dockerfile` para
mantener la coherencia, ampliando el diff sin necesidad.

### D3 — El `.dockerignore` pierde la regla de `package-lock.json`

La regla `frontend/package-lock.json` existe solo para no meter en el contexto de build un
fichero que, tras este cambio, ya no existe. Se retira para que el fichero no documente un
estado falso. Se conserva intacta la exclusión de `frontend/node_modules/`, que sigue siendo
necesaria por una razón distinta y documentada.

### D4 — La "fase RED" es de verificación, no de tests nuevos

No hay comportamiento de aplicación que testear: el cambio es de configuración y
documentación. La evidencia de fallo previo se obtiene con comandos que hoy demuestran el
estado indeseado (dos lockfiles presentes, auditoría duplicada), y la evidencia de éxito con
los comandos de la sección de verificación. `tasks.md` refleja ese enfoque.

## Risks / Trade-offs

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Dependabot deja de actualizar el frontend | Medio — dependencias sin parches | Sin `dependabot.yml` la config es de la UI de GitHub. Verificar tras el merge que el ecosistema `npm` de `/frontend` sigue resolviendo contra `pnpm-lock.yaml`, y si no, reconfigurarlo. Escenario explícito en el spec. |
| Divergencia de versión de pnpm local vs Docker | Bajo | `packageManager` fija `11.21.0` (D2) y el lockfile está validado con 11.x y 12.x. |
| El contrato del frontend no se ejecuta en CI | Medio — la regresión pasaría desapercibida | Se asume como Non-Goal y se registra como follow-up: añadir un job de frontend a `ci.yml` con pnpm. El cambio actual no lo empeora: hoy tampoco existe. |
| Contributors con `npm install` en el frontend | Bajo | Se actualizan `README.md` y `AGENTS.md` en el mismo cambio para que la instrucción vigente sea pnpm. |
