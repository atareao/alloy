# Tasks — manual-check-summary

## 1. RED (tests que fallan)

- [x] 1.1 Backend: test de que el evento de fin (`done = true`) transporta los contadores reales
      (`checked`, `updatable`, `uptodate`, `updated`, `failed`, `errors`).
- [x] 1.2 Backend: test de que `reset_progress_cache` NO emite `done = true` con todos los contadores a cero.
- [x] 1.3 Frontend: test de `mapBatchToResults(bp)` (mapeo de los 6 contadores a `check`/`update`).
- [x] 1.4 Frontend: test de `SummaryDialog` que renderiza Comprobados/Actualizables/Actuales/Errores y Actualizados/Fallos con datos reales.

## 2. GREEN (implementación mínima)

- [x] 2.1 `models.rs`: añadir `updatable`, `uptodate`, `failed` a `BatchProgress` y `UpdateProgress` (`serde(default)`).
- [x] 2.2 `handlers.rs::check_and_apply_all`: contabilizar `updatable`, `uptodate`, `failed`; emitir `done=true` con contadores reales (checked efectivo) antes de limpiar la caché.
- [x] 2.3 `handlers.rs::reset_progress_cache`: dejar de emitir `done=true` con ceros (idle `done=false`).
- [x] 2.4 `App.tsx::finishBatch`: usar `mapBatchToResults` para rellenar `checkResults`/`updateResults`.
- [x] 2.5 `types.ts`: ampliar `BatchProgress` y `UpdateProgress` con los campos nuevos; añadir `mapBatchToResults`.
- [x] 2.6 `SummaryDialog.tsx`: consumir los nuevos valores (sin cambios de estructura si el mapeo es correcto).

## 3. REFACTOR y verificación

- [x] 3.1 `cd backend && just check` (fmt + clippy).
- [x] 3.2 `cargo test` backend en verde.
- [x] 3.3 `cd frontend && pnpm run build` y `pnpm exec tsc -b`; `pnpm test` (vitest) en verde.
- [x] 3.4 `pnpm run lint`.

## 4. Cierre

- [x] 4.1 Auditar (rust-reviewer + react-reviewer), PR a `develop`, CI verde, `openspec archive manual-check-summary`.
