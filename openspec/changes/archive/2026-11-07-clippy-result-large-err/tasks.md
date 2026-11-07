# Tasks

## 1. RED

- [x] 1.1 Reproducir el fallo: `cargo +1.99.1 clippy -- -D warnings` → 3 errores `result_large_err` en `auth.rs`.

## 2. GREEN

- [x] 2.1 `backend/src/main.rs`: `#![allow(clippy::result_large_err)]` con comentario.
- [x] 2.2 `cargo +1.99.1 clippy -- -D warnings` → limpio.

## 3. REFACTOR / Verificación

- [x] 3.1 `cargo test` sin regresiones (300 OK; 4 integración se autosaltan sin Podman).
- [x] 3.2 `cargo fmt -- --check` OK.

## 4. Archive

- [x] 4.1 `openspec archive clippy-result-large-err --yes` + limpieza.
