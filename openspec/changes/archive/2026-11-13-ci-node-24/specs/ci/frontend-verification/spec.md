# Spec Delta — ci/frontend-verification

## MODIFIED Requirements

### Requirement: Coherencia de toolchain entre CI y build de producción

El job de frontend MUST usar la misma versión mayor de Node que el stage `frontend-builder` del
`Dockerfile`, y esa versión MUST ser la **LTS vigente**. La versión MUST declararse en una
**fuente única de verdad** — el campo `engines.node` de `frontend/package.json` — y el workflow
MUST obtenerla de ahí (`node-version-file`), sin declarar una `node-version` propia. El workflow
MUST obtener la versión de pnpm del campo `packageManager` de `frontend/package.json` en lugar
de declararla por duplicado.

#### Scenario: Versión de pnpm de una única fuente

- **Given** el campo `packageManager` de `frontend/package.json`
- **When** el workflow prepara pnpm
- **Then** la versión instalada deriva de ese campo
- **And** el workflow no declara una versión de pnpm propia

#### Scenario: Versión de Node de una única fuente

- **Given** el campo `engines.node` de `frontend/package.json` que resuelve al major 24
- **When** el job de frontend configura Node
- **Then** instala una versión que satisface ese rango (major 24)
- **And** el workflow no declara una `node-version` literal

#### Scenario: Node alineado con el Dockerfile

- **Given** que el `Dockerfile` construye el frontend sobre `node:24-alpine`
- **When** el job de frontend configura Node
- **Then** usa la versión mayor 24

#### Scenario: Deriva entre el Dockerfile y engines.node

- **Given** un `Dockerfile` cuyo stage `frontend-builder` usa un major de Node distinto del de
  `engines.node` en `frontend/package.json`
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de comprobación de consistencia
