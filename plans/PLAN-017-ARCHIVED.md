# PLAN-017 — Actualizar README (#160)

**Issue:** #160
**Change OpenSpec:** `openspec/changes/readme-update/`
**Rama:** `feature/readme-update` (desde `develop`)

## Objetivo

Alinear `README.md` con la stack real: React 19 + Ant Design 6 + Vite 8 + TypeScript 6, persistencia SQLite (`rusqlite` + `deadpool-sqlite`, `data/alloy.db`), módulos reales del backend y cifras de tests actuales.

## Mapa de cambios

- Badges (líneas 7-15).
- Nota introductoria (línea 26).
- Diagrama de arquitectura (línea 87).
- Tabla de módulos (líneas 111-127).
- Árbol de proyecto (líneas 440-475).
- Sección de testing (líneas 419-433).
- Tabla de tech stack (líneas 489-498).
- Instrucciones de desarrollo (líneas 480-485).

## Tareas

1. Actualizar badges y nota introductoria.
2. Actualizar diagrama, tabla de módulos y árbol de proyecto.
3. Actualizar sección de testing y tech stack.
4. Corregir instrucciones de desarrollo.
5. Verificar con `rg` que no quedan términos obsoletos.
6. Revisar diff, commit y PR a `develop`.

## Criterios de aceptación

- [x] No aparecen React 18, Mantine, JSON files, persistence.rs, 321 tests, 81 tests ni 62_passing.
- [x] Aparecen React 19, Ant Design 6, SQLite, `data/alloy.db`, 492 y 135.
- [x] El árbol de proyecto coincide con `backend/src/`.
- [x] Diff limitado a `README.md` (y artefactos OpenSpec/plan).
- [x] PR a `develop` con referencia a #160.
