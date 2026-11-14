# Proposal

## Why

Los 12 tests `test_integration_*` del backend se conectan al socket de Podman y asumen
contenedores concretos de la máquina del desarrollador. Hoy evitan fallar con un
early-return manual, así que `cargo test` los cuenta como **passed** sin ejecutar nada
(falso verde) y, cuando el socket existe pero la conexión falla, el test revienta por un
`.expect`. Además no hay manera estándar de ejecutarlos a demanda y en CI nunca validan nada.

## What Changes

- Marcar los 12 tests `test_integration_*` con `#[ignore = "…"]` y eliminar los early-returns
  manuales (`if !is_podman_available() { eprintln!("SKIP…"); return; }`).
- Añadir un módulo compartido `backend/src/test_support.rs` (solo `cfg(test)`) con la
  resolución única del socket: `DOCKER_HOST` → `XDG_RUNTIME_DIR/podman/podman.sock` →
  `/run/podman/podman.sock` → fallback legacy `/run/user/1000/podman/podman.sock`.
  Los helpers duplicados en `containers.rs` y `stacks.rs` se reemplazan por ese módulo.
- `podman_client()` deja de ser `async` (no lo era realmente) y falla con un mensaje
  accionable en vez de un panic opaco.
- Nuevas recipes just: `test` (unitarios, por defecto) y `test-integration`
  (`cargo test -- --ignored`, con preflight del socket y mensaje claro si no existe).
- CI sin cambios de comportamiento: sigue ejecutando `cargo test` (unitarios) y ahora
  informa explícitamente cuántos tests quedan ignorados.
- Documentación `docs/integration-tests.md` con el procedimiento local.

Sin cambios **BREAKING**: no se modifica código de producción ni la API.

## Capabilities

### New Capabilities

- `backend/integration-tests`: contrato de la suite de tests del backend — `cargo test`
  MUST pasar sin daemon de contenedores, los tests que requieren Podman MUST declararse con
  `#[ignore]` y ejecutarse solo bajo demanda (`--ignored` / `just test-integration`).

### Modified Capabilities

- Ninguna. `backend/lint-hygiene` (fmt/clippy) no cambia sus requisitos.

## Impact

- `backend/src/containers.rs`, `backend/src/stacks.rs` (atributos de test + eliminación de helpers).
- `backend/src/lib.rs` o `backend/src/main.rs` (declaración del módulo `test_support` bajo `cfg(test)`).
- `backend/src/test_support.rs` (nuevo).
- `.justfile` (raíz; recipes `test`, `test-integration` con preflight de socket).
- `.github/workflows/ci.yml` (salida informativa del recuento de ignorados; sin cambio de alcance).
- `docs/integration-tests.md` (nuevo), `README.md` (referencia opcional).
