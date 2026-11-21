# frontend/deps Specification

## Purpose
TBD - created by archiving change deps-minor. Update Purpose after archive.

## Requirements

### Requirement: Las dependencias del frontend SHALL actualizarse a parches/menores

`frontend/package.json` y `frontend/pnpm-lock.yaml` MUST quedar con las versiones indicadas en el proposal. Ninguna dependencia MUST subir de major en este change.

#### Scenario: Actualización dentro de rango

- **GIVEN** `frontend/package.json`
- **WHEN** se ejecuta `pnpm update` en `frontend/`
- **THEN** `antd`, `react`, `react-dom`, `vite`, `oxlint`, `@vitejs/plugin-react`, `@testing-library/react`, `@types/react` y `@types/react-dom` quedan en las versiones del proposal
- **AND** `vitest` permanece en la serie 4, `jsdom` en la 29, `typescript` en la 6.0 y `@testing-library/jest-dom` en la 6

### Requirement: La suite SHALL seguir verde tras la actualización

Tras actualizar, `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint` y `pnpm run build` MUST terminar con código 0 en `frontend/`.

#### Scenario: Verificación completa

- **GIVEN** las dependencias actualizadas
- **WHEN** se ejecutan `pnpm exec tsc -b`, `pnpm run test`, `pnpm run lint` y `pnpm run build`
- **THEN** los cuatro terminan con código 0
- **AND** el número de tests no disminuye respecto al baseline (174)

### Requirement: El lockfile SHALL quedar consistente

`pnpm install --frozen-lockfile` MUST funcionar sin cambios, garantizando que `pnpm-lock.yaml` está sincronizado con `package.json`.

#### Scenario: Lockfile sincronizado

- **GIVEN** el repositorio con el lockfile actualizado
- **WHEN** se ejecuta `pnpm install --frozen-lockfile` en `frontend/`
- **THEN** el comando termina con código 0 y no modifica el lockfile
