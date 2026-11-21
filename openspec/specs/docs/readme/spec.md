# docs/readme Specification

## Purpose
TBD - created by archiving change readme-update. Update Purpose after archive.

## Requirements

### Requirement: El README SHALL reflejar la stack real

El `README.md` MUST indicar React 19, TypeScript 6, Vite 8 y Ant Design 6 para el frontend. MUST indicar Rust/Axum 0.8, Tokio y Bollard para el backend. MUST indicar OIDC como único mecanismo de autenticación.

#### Scenario: Badges y tech stack correctos

- **GIVEN** el `README.md`
- **WHEN** se revisan los badges y la tabla de tech stack
- **THEN** aparecen React 19, TypeScript 6 y Ant Design 6
- **AND** no aparece React 18 ni Mantine

### Requirement: El README SHALL documentar la persistencia SQLite

El `README.md` MUST indicar que el estado se persiste en SQLite mediante `rusqlite` + `deadpool-sqlite`, en la base `data/alloy.db`. MUST NOT afirmar que la persistencia es en ficheros JSON ni que no se requiere base de datos.

#### Scenario: Persistencia correcta

- **GIVEN** el `README.md`
- **WHEN** se revisan la nota introductoria, el árbol de proyecto y la tabla de tech stack
- **THEN** se menciona SQLite y `data/alloy.db`
- **AND** no se mencionan ficheros JSON ni `persistence.rs`

### Requirement: El README SHALL documentar los módulos y tests reales

El `README.md` MUST listar los módulos reales del backend (`db.rs`, `updates/`, `workers/`, `timezone.rs`, `test_support.rs`, etc.) y MUST NOT referenciar `persistence.rs`, `workers.rs` ni `updates.rs` como ficheros únicos. La sección de testing MUST indicar 492 tests backend y 135 frontend, con 12 tests de integración ignorados.

#### Scenario: Módulos y tests actualizados

- **GIVEN** el `README.md`
- **WHEN** se revisan la tabla de módulos, el árbol de proyecto y la sección de testing
- **THEN** los módulos coinciden con `backend/src/`
- **AND** las cifras son 492 backend y 135 frontend
- **AND** no aparecen 321, 81 ni 62 como recuentos de tests
