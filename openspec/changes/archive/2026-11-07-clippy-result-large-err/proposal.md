# Proposal

## Why

CI (Rust stable = 1.99.0) falla en `cargo clippy -- -D warnings` con 3 errores
`clippy::result_large_err` en `backend/src/auth.rs` (líneas 28, 57, 311). El lint no existía
con el toolchain anterior (1.97.0) y **bloquea todos los PRs** del repo, incluidos los de
Dependabot. No es un problema de ninguna dependencia.

## What Changes

- `backend/src/main.rs`: allow a nivel de crate `#![allow(clippy::result_large_err)]`, con
  comentario justificativo. Los handlers axum devuelven `Result<T, Response>` por diseño
  (`Response` = 128 bytes); boxear el error rompería el contrato `IntoResponse` y es el
  anti-patrón que la propia comunidad axum resuelve con este allow.

## Capabilities

### New Capabilities
- `backend/lint-hygiene`: el backend compila limpio con `cargo clippy -- -D warnings` en el
  toolchain stable actual.

### Modified Capabilities
<!-- Ninguna. -->

## Impact

- `backend/src/main.rs` (una línea + comentario). Sin cambio de comportamiento.
- Sin cambios de API ni de contrato.
