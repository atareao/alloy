# Tasks

## Checklist

### README

- [x] Actualizar badges: React 19, TypeScript 6, Ant Design 6, tests 627.
- [x] Corregir la nota introductoria: SQLite (`data/alloy.db`), no JSON.
- [x] Actualizar el diagrama de arquitectura: React + Ant Design.
- [x] Actualizar la tabla de módulos del backend (módulos reales, sin líneas obsoletas).
- [x] Actualizar el árbol de proyecto (`db.rs`, `updates/`, `workers/`, `timezone.rs`, `test_support.rs`, `data/alloy.db`).
- [x] Actualizar la sección de testing: 492 backend + 135 frontend, 12 ignorados.
- [x] Actualizar la tabla de tech stack: React 19, TypeScript 6, Vite 8, Ant Design 6, SQLite.
- [x] Corregir instrucciones de desarrollo (`load_json`/`json_writer`, `workers.rs`).

### Verificación

- [x] `rg -n "React 18|Mantine|JSON files|persistence\\.rs|321 tests|81 tests|62_passing" README.md` no devuelve resultados.
- [x] `rg -n "React 19|Ant Design|SQLite|data/alloy\\.db|492|135" README.md` devuelve las menciones esperadas.
- [x] Revisar el diff completo de `README.md`.
- [x] Commit con gitmoji y PR a `develop`.
