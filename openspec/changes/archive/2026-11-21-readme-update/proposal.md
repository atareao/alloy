# Proposal

## Why

El `README.md` describe una stack que ya no existe: afirma React 18, Mantine UI 7 y persistencia en ficheros JSON. La realidad es React 19, Ant Design 6 y SQLite (`rusqlite` + `deadpool-sqlite`, base `data/alloy.db`). También hay cifras de tests y referencias a módulos obsoletas (`persistence.rs`, `workers.rs`, `updates.rs`). Esto confunde a usuarios y contribuidores.

## What Changes

- Actualizar badges: React 19, TypeScript 6, Ant Design 6 y recuento de tests.
- Corregir la nota introductoria: persistencia SQLite, no JSON.
- Actualizar el diagrama de arquitectura: React + Ant Design.
- Actualizar la tabla de módulos del backend a los módulos reales, sin recuentos de líneas obsoletos.
- Actualizar el árbol de proyecto: `db.rs`, `updates/`, `workers/`, `timezone.rs`, `test_support.rs`; `data/alloy.db`.
- Actualizar la sección de testing: 492 tests backend + 135 frontend, 12 integración ignorados.
- Actualizar la tabla de tech stack: React 19, TypeScript 6, Vite 8, Ant Design 6, SQLite.
- Corregir instrucciones de desarrollo que mencionan `load_json`/`json_writer` y `workers.rs`.

## Capabilities

### New Capabilities

- `docs/readme`: el README debe reflejar la stack y persistencia reales.

### Modified Capabilities

Ninguna.

## Impact

- Solo `README.md`. Sin cambios de código, dependencias ni API.
- Issue: #160.
