# Tasks

## 1. Backend: Pasar contadores reales a apply_single_policy

- [x] 1.1 Añadir parámetros `total: u32`, `checked: u32`, `updated: u32`, `errors: u32` a `apply_single_policy`
- [x] 1.2 Actualizar llamada a `apply_single_policy` desde `check_and_apply_all` para pasar los contadores
- [x] 1.3 Reemplazar todos los `0` en llamadas a `update_progress` dentro de `apply_single_policy` con los parámetros recibidos
- [x] 1.4 Verificar que `cargo check` compila sin errores
- [x] 1.5 Verificar que `cargo test` pasa todos los tests

## 2. Frontend: Añadir console.log en SSE handler

- [x] 2.1 Añadir `console.log("SSE update-progress:", data)` en el manejador de eventos `update-progress` en `App.tsx`
- [x] 2.2 Verificar que `npx tsc --noEmit` compila sin errores

## 3. Frontend: Arreglar 0 ?? current en BatchProgress.tsx

- [x] 3.1 Cambiar `const checked = latestProgress?.checked ?? current;` por una comprobación explícita de null/undefined
- [x] 3.2 Verificar que `npx vitest run` pasa todos los tests

## 4. Verificación

- [x] 4.1 Ejecutar `cargo clippy -- -D warnings` en backend
- [x] 4.2 Ejecutar `cargo test` en backend
- [x] 4.3 Ejecutar `npx vitest run` en frontend
- [x] 4.4 Ejecutar `npx tsc --noEmit` en frontend