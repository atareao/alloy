# backend/lint-hygiene Specification

## Purpose
Garantizar que el backend de Alloy pase `cargo clippy -- -D warnings` en el toolchain Rust
stable actual (1.99.x), sin bloquear CI por lints de estilo inherentes al patrón de axum.

## Requirements

### Requirement: Clippy limpio en stable actual

`cd backend && cargo clippy -- -D warnings` MUST terminar sin errores ni warnings en el
toolchain stable actual. El lint `clippy::result_large_err` MUST estar permitido a nivel de
crate, dado que los handlers axum devuelven `Result<T, Response>` por diseño.

#### Scenario: Clippy en CI verde

- **Given** el toolchain stable actual (Rust 1.99.x)
- **When** se ejecuta `cargo clippy -- -D warnings` en `backend/`
- **Then** termina con éxito (exit 0) y sin warnings

#### Scenario: Sin cambio de comportamiento

- **Given** el allow añadido
- **When** se ejecuta `cargo test` en `backend/`
- **Then** todos los tests siguen pasando igual que antes
