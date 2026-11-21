# frontend/deps Specification

## Purpose
TBD - created by archiving change deps-minor. Update Purpose after archive.

## Requirements

### Requirement: Las dependencias del frontend SHALL actualizarse a parches/menores

`frontend/package.json` y `frontend/pnpm-lock.yaml` MUST quedar con las versiones indicadas en el proposal. Este change SÍ sube de major: `vitest` a la serie 5, `jsdom` a la 31, `typescript` a la 7.0 y `@testing-library/jest-dom` a la 8.

#### Scenario: Actualización dentro de rango

- **GIVEN** `frontend/package.json`
- **WHEN** se revisan las dependencias
- **THEN** `vitest` es `^5.1.1`, `jsdom` es `^31.0.1`, `typescript` es `~7.0.2`, `@testing-library/jest-dom` es `^8.0.0`
- **AND** `@types/node` sigue en la serie 24

### Requirement: La suite SHALL seguir verde tras la actualización

Tras actualizar, `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint` y `pnpm run build` MUST terminar con código 0 en `frontend/`.

#### Scenario: Verificación completa

- **GIVEN** las dependencias actualizadas
- **WHEN** se ejecutan `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint` y `pnpm run build`
- **THEN** los cuatro terminan con código 0
- **AND** el número de tests no disminuye respecto al baseline (174)

### Requirement: El lockfile SHALL quedar consistente

`pnpm install --frozen-lockfile` MUST terminar con código 0 sin modificar `frontend/pnpm-lock.yaml`. Tras los majors, el lockfile MUST reflejar `vitest` 5, `jsdom` 31, `typescript` 7 y `@testing-library/jest-dom` 8.

#### Scenario: Lockfile sincronizado

- **GIVEN** el lockfile actualizado con los majors
- **WHEN** se ejecuta `pnpm install --frozen-lockfile`
- **THEN** termina con código 0 y no modifica el lockfile

### Requirement: Los majors del frontend SHALL actualizarse

`frontend/package.json` MUST declarar `vitest` en la serie 5, `jsdom` en la 31, `typescript` en la 7 y `@testing-library/jest-dom` en la 8. MUST declarar `@testing-library/dom` como devDependency. `@types/node` MUST permanecer en la serie 24.

#### Scenario: Versiones objetivo

- **GIVEN** `frontend/package.json`
- **WHEN** se revisan las dependencias
- **THEN** `vitest` es `^5.1.1`, `jsdom` es `^31.0.1`, `typescript` es `~7.0.2`, `@testing-library/jest-dom` es `^8.0.0`
- **AND** `@testing-library/dom` figura en devDependencies
- **AND** `@types/node` sigue en la serie 24

### Requirement: La suite SHALL seguir verde con los majors

Tras la actualización, `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint` y `pnpm run build` MUST terminar con código 0, y el recuento de tests MUST ser mayor o igual a 174.

#### Scenario: Verificación completa

- **GIVEN** las dependencias actualizadas
- **WHEN** se ejecutan `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint` y `pnpm run build`
- **THEN** los cuatro terminan con código 0
- **AND** hay al menos 174 tests

### Requirement: La alineación de Node SHALL mantenerse

`engines.node` MUST exigir al menos la versión 24.15 y MUST seguir siendo coherente con el major 24 del Dockerfile.

#### Scenario: Node alineado

- **GIVEN** `frontend/package.json` y el `Dockerfile`
- **WHEN** se comparan los majors de Node
- **THEN** ambos son 24
