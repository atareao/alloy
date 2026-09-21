# Tasks

## TDD Checklist

### Frontend

- [x] ELIMINAR: polling `useEffect` (App.tsx líneas 248-283)
- [x] AÑADIR: `useEffect` de recuperación única al montar vía `GET /api/check-progress`
- [x] VERIFICAR: `npx vitest run` pasa (37 tests, incluidos 2 nuevos)
- [x] VERIFICAR: `npx tsc -b` sin errores
- [x] VERIFICAR: `npm run lint` sin errores

### Backend

- [x] Sin cambios en backend (solo lectura del endpoint existente)
- [x] VERIFICAR: `cargo test` pasa (192 passed, 4 integration skipped)
- [x] VERIFICAR: `cargo clippy -- -D warnings` sin errores
- [x] VERIFICAR: `cargo fmt --check` sin errores