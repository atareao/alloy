# Tasks

## 1. RED — Tests que describen el contrato

- [x] 1.1 Añadir en `backend/src/test_support.rs` los tests unitarios de `resolve_socket()`
      (función pura): prioridad de `DOCKER_HOST`, `XDG_RUNTIME_DIR` (con y sin socket
      presente), socket del sistema, fallback legacy, barra final y valores en blanco.
- [x] 1.2 Ejecutar `cargo test` y confirmar que fallan (el módulo aún no existe / no resuelve).
- [x] 1.3 Línea base confirmada: sin daemon los `test_integration_*` se contaban como pasados
      (falso verde) y `cargo test -- --ignored` no ejecutaba ninguno; en esta máquina 4 de
      ellos fallaban por contenedores locales ausentes.

## 2. GREEN — Implementación mínima

- [x] 2.1 Crear `backend/src/test_support.rs` con `podman_socket`, `is_podman_available`,
      `podman_client` (síncrono, error accionable) y registrar el módulo con `#[cfg(test)]`.
- [x] 2.2 Sustituir los helpers duplicados de `containers.rs` y `stacks.rs` por el módulo común.
- [x] 2.3 Marcar los 12 `test_integration_*` con `#[ignore = "…"]` y borrar los early-returns.
- [x] 2.4 Añadir `test` y `test-integration` al `.justfile` (raíz) con preflight de socket.
- [x] 2.5 `cargo test` → verde con `12 ignored`; `cargo clippy -- -D warnings` y `cargo fmt --check` limpias.

## 3. REFACTOR

- [x] 3.1 Eliminar imports y helpers que queden sin uso; revisar `cargo clippy --all-targets`.
- [x] 3.2 Paso informativo+guardia del recuento de ignorados en `.github/workflows/ci.yml`.
- [x] 3.3 `just test-integration` en local (con daemon) y comprobación del aborto sin daemon.

## 4. Documentación y cierre

- [x] 4.1 `docs/integration-tests.md`: qué cubren, cuándo correrlos, preflight y limitaciones.
- [x] 4.2 Referencia en `README.md`.
- [ ] 4.3 Auditar con `@rust-reviewer` (hecho, rechazado → corregido), abrir PR a `develop`,
      archivar el change con `openspec archive`.
