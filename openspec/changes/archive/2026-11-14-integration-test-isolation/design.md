# Design

## Contexto

12 tests `test_integration_*` (10 en `containers.rs`, 2 en `stacks.rs`) se conectan al
socket de Podman vía `bollard` y asumen contenedores reales del entorno del desarrollador
(`alloy`, `oxinbox`, `preemer-db`). El escape actual es un early-return manual que los
convierte en falsos positivos dentro de `cargo test`, y `podman_client()` hace
`.expect(...)`, que convierte un socket inaccesible en un panic en vez de un skip legible.

`backend/tests/` no existe: todos los tests viven en módulos `#[cfg(test)]`.

## Decisiones

### D1 — `#[ignore]` en lugar de feature flag de Cargo

Se descarta un feature `integration` porque:
- obliga a mantener dos configuraciones de compilación y a recordar `--features`;
- `cargo test -- --ignored` es el mecanismo idiomático de Rust para tests dependientes de
  entorno y no añade superficie de configuración;
- permite seguir compilando los tests siempre (detecta roturas de API en cada `cargo test`).

`#[ignore = "requires a running Podman daemon and the local containers (alloy, oxinbox, preemer-db)"]`
documenta la razón en el propio resumen de la suite.

Se elimina el early-return manual: con `#[ignore]` es código muerto y, si alguien invoca
`--ignored` en un entorno sin daemon, el fallo debe ser explícito, no silencioso.

### D2 — Módulo `backend/src/test_support.rs`

Helper único, compilado solo en tests:

```rust
#[cfg(test)]
pub mod test_support;
```

Contenido: `resolve_socket(docker_host, xdg_runtime_dir, socket_exists) -> String`,
`podman_socket() -> String`, `has_usable_socket(socket)` (privado) y
`podman_client() -> Docker` (síncrono, sin `async`); `test_pool()` sigue en `db.rs`.

`resolve_socket` es puro: recibe los valores de entorno y un predicado
`socket_exists(&str) -> bool`, de modo que la cascada se testea sin depender del sistema
de ficheros ni de mutar variables de entorno (que en Rust 2024 es `unsafe` y hace los
tests no deterministas al ser proceso-global).

Resolución del socket, en orden:

1. `DOCKER_HOST`
2. `${XDG_RUNTIME_DIR}/podman/podman.sock` **solo si el fichero existe** (en un host
   rootful `XDG_RUNTIME_DIR` está definido pero no hay socket de usuario: sin esta
   comprobación nunca se llegaría al socket del sistema)
3. `/run/podman/podman.sock` si existe
4. `/run/user/1000/podman/podman.sock` (legacy, sin comprobar existencia)

Esta cascada elimina el UID 1000 hardcodeado y hace que funcione en rootless (UID
arbitrario), root y Docker Desktop/CI.

### D3 — Recipes `just`

```make
test:
    cd backend && cargo test

test-integration:
    cd backend && cargo test -- --ignored
```

`test-integration` precede la ejecución con un preflight que, si no hay socket,
imprime un error accionable y sale con código != 0 antes de invocar cargo. Con
`--ignored`, cargo ejecuta *solo* los tests ignorados, así que no re-ejecuta los unitarios.

### D4 — CI

`cargo test` sigue siendo el único paso (sin daemon en el runner, y los tests dependen de
contenedores locales concretos: un job real de integración sería no determinista y no se
puede aprovisionar de forma fiable). Se añade un paso informativo que imprime la línea
`test result: ... N ignored` para que el aislamiento sea visible en los logs y una
regresión (tests que dejan de estar ignorados) salte a la vista.

Nota: `ubuntu-latest` no tiene Podman y sus contenedores no son los esperados, por lo que
`--ignored` en CI queda explícitamente fuera de alcance.

## Riesgos

- **Cobertura aparente**: al marcar como ignorados, la cobertura "verde" de `cargo test`
  baja de 12 tests falsos a 0. Es el objetivo; los unitarios reales siguen cubriendo la
  lógica pura.
- **Rotura silenciosa de los tests de integración**: al no correr en CI pueden quedar
  obsoletos. Mitigación: `just test-integration` en la checklist de release y mención en
  `docs/integration-tests.md`.
