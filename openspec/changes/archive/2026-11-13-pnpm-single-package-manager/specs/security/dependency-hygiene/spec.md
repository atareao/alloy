# Spec Delta

## REMOVED Requirements

### Requirement: Lockfiles sin vulnerabilidades parcheables

**Reason**: El requirement nombraba explícitamente `frontend/package-lock.json` como lockfile
auditable y prescribía `npm audit` como comando de verificación. Ambos desaparecen al
unificar el gestor de paquetes del frontend en pnpm, así que el texto deja de describir un
estado realizable.

**Migration**: Sustituido en este mismo cambio por dos requirements: *"Lockfiles pnpm sin
vulnerabilidades parcheables"* (mismos mínimos de versión, auditoría vía `pnpm audit` sobre
`pnpm-lock.yaml`) y *"Gestor de paquetes único en el frontend"* (invariantes del gestor
único). No requiere acción de los consumidores: los mínimos de versión se preservan sin
cambios.

## MODIFIED Requirements

### Requirement: Sin regresión funcional

La unificación del gestor de paquetes MUST NOT introducir cambios de comportamiento en la
aplicación. La suite de tests y el build MUST seguir en verde.

La verificación de este requirement se limita a los tests que no dependen de infraestructura:
los tests de integración de `containers` que exigen un daemon Docker con los contenedores
`alloy`/`oxinbox` en ejecución quedan fuera de su alcance.

#### Scenario: Backend en verde

- **Given** `backend/Cargo.lock` sin cambios
- **When** se ejecuta `cargo clippy -- -D warnings` en `backend/`
- **Then** compila sin warnings
- **And** la suite de tests unitarios pasa

#### Scenario: Frontend en verde

- **Given** `frontend/pnpm-lock.yaml` como único lockfile del frontend
- **When** se ejecuta `pnpm exec tsc -b`, `pnpm run test` y `pnpm run build` en `frontend/`
- **Then** type-check, tests y build pasan sin errores

#### Scenario: Build Docker consistente

- **Given** `frontend/package-lock.json` eliminado
- **When** el stage `frontend-builder` ejecuta `pnpm install --frozen-lockfile` y `pnpm run build`
- **Then** el build completa sin error de lockfile desincronizado
- **And** el contexto de build no declara la exclusión de ningún fichero inexistente

## ADDED Requirements

### Requirement: Lockfiles pnpm sin vulnerabilidades parcheables

Los lockfiles del proyecto (`frontend/pnpm-lock.yaml`, `backend/Cargo.lock`) MUST NOT fijar
dependencias con advisory abierto de severidad `low`, `medium` o `high` cuando exista una
versión parcheada dentro de un rango permitido por las dependencias que las introducen. El
frontend MUST tener exactamente un lockfile, `frontend/pnpm-lock.yaml`. En particular, tras
este cambio:

- `undici` MUST ser `>= 7.30.1` (introducida por `jsdom`).
- `vite` MUST ser `>= 8.1.6` (devDependency directa).
- `@vitest/mocker` MUST ser `>= 4.1.12` (introducida por `vitest`).
- `postcss` MUST ser `>= 8.5.23` (introducida por `vite`).
- `source-map-js` MUST ser `>= 1.2.2` (introducida por `postcss`/`css-tree`).
- `rustls` MUST ser `>= 0.23.46` (introducida por `bollard`/`reqwest`).

#### Scenario: Auditoría pnpm limpia

- **Given** `frontend/pnpm-lock.yaml` como único lockfile del frontend
- **When** se ejecuta `pnpm audit` en `frontend/`
- **Then** el reporte no muestra vulnerabilidades abiertas
- **And** las versiones fijadas cumplen los mínimos anteriores

#### Scenario: Aviso Dependabot resuelto

- **Given** el estado tras el cambio
- **When** se consulta `gh api repos/atareao/alloy/dependabot/alerts`
- **Then** no quedan alertas en estado `open` para los paquetes listados

#### Scenario: Dependabot sigue versionando el frontend

- **Given** `frontend/package-lock.json` eliminado y `frontend/pnpm-lock.yaml` presente
- **When** Dependabot analiza el ecosistema `npm` de `/frontend`
- **Then** sigue detectando las dependencias declaradas en `frontend/package.json`
- **And** los avisos se resuelven contra `frontend/pnpm-lock.yaml`

### Requirement: Gestor de paquetes único en el frontend

El proyecto MUST usar pnpm como único gestor de paquetes del frontend. `frontend/` MUST
contener exactamente un lockfile (`pnpm-lock.yaml`) y MUST NOT contener `package-lock.json`
ni `yarn.lock`. `frontend/package.json` MUST declarar el campo `packageManager` con la misma
versión mayor que la que fija el stage `frontend-builder` del `Dockerfile`, de modo que el
entorno local y el de build no puedan divergir.

#### Scenario: Repositorio sin lockfile npm

- **Given** el árbol de trabajo en `frontend/`
- **When** se listan los lockfiles
- **Then** existe `pnpm-lock.yaml`
- **And** no existe `package-lock.json`

#### Scenario: Instalación reproducible

- **Given** `frontend/pnpm-lock.yaml` sincronizado con `frontend/package.json`
- **When** se ejecuta `pnpm install --frozen-lockfile`
- **Then** la instalación completa sin error
- **And** el lockfile no se modifica

#### Scenario: Versión de gestor coherente

- **Given** el campo `packageManager` de `frontend/package.json`
- **When** se compara con el pin de pnpm del stage `frontend-builder` del `Dockerfile`
- **Then** la versión mayor coincide
- **And** el lockfile es aceptado por esa versión con `--frozen-lockfile`
