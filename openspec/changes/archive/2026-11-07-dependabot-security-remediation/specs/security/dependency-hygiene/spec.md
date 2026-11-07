# Spec Delta

## Purpose

Garantizar que los lockfiles de Alloy (`frontend/package-lock.json`, `frontend/pnpm-lock.yaml`,
`backend/Cargo.lock`) no fijen dependencias con vulnerabilidades conocidas que ya tengan versión
parcheada publicada, sin alterar el comportamiento de la aplicación.

## ADDED Requirements

### Requirement: Lockfiles sin vulnerabilidades parcheables

Los lockfiles del proyecto MUST NOT fijar dependencias con advisory abierto de severidad `low`,
`medium` o `high` cuando exista una versión parcheada dentro de un rango permitido por las
dependencias que las introducen. En particular, tras este cambio:

- `undici` MUST ser `>= 7.30.1` (introducida por `jsdom`).
- `vite` MUST ser `>= 8.1.6` (devDependency directa).
- `@vitest/mocker` MUST ser `>= 4.1.12` (introducida por `vitest`).
- `postcss` MUST ser `>= 8.5.23` (introducida por `vite`).
- `source-map-js` MUST ser `>= 1.2.2` (introducida por `postcss`/`css-tree`).
- `rustls` MUST ser `>= 0.23.46` (introducida por `bollard`/`reqwest`).

#### Scenario: Auditoría npm limpia

- **Given** los lockfiles regenerados
- **When** se ejecuta `npm audit` en `frontend/`
- **Then** el reporte no muestra vulnerabilidades abiertas (0 vulnerabilities)
- **And** las versiones fijadas cumplen los mínimos anteriores

#### Scenario: Aviso Dependabot resuelto

- **Given** el estado tras el cambio
- **When** se consulta `gh api repos/atareao/alloy/dependabot/alerts`
- **Then** no quedan alertas en estado `open` para los paquetes listados

### Requirement: Sin regresión funcional

Las actualizaciones MUST ser bumps minor/patch sin cambios de código de aplicación. La suite
debe seguir en verde.

#### Scenario: Backend en verde

- **Given** `Cargo.lock` actualizado (rustls 0.23.46)
- **When** se ejecuta `cargo test` y `cargo clippy -- -D warnings` en `backend/`
- **Then** compila sin warnings y todos los tests pasan

#### Scenario: Frontend en verde

- **Given** los lockfiles npm/pnpm actualizados
- **When** se ejecuta `npx tsc -b`, `npx vitest run` y `npm run build` en `frontend/`
- **Then** type-check, tests y build pasan sin errores

#### Scenario: Build Docker consistente

- **Given** `pnpm-lock.yaml` regenerado
- **When** el stage `frontend-builder` ejecuta `pnpm install --frozen-lockfile` y `pnpm run build`
- **Then** el build completa sin error de lockfile desincronizado
