# Tests de integración

## Qué es

Alloy incluye 12 tests que hablan con el **socket del daemon de contenedores**
(Podman o Docker) y que, por tanto, dependen del entorno real de la máquina:
necesitan que el daemon esté accesible y que existan contenedores concretos.

Para evitar **falsos verdes** (ejecutarlos sin daemon daría un `skip` silencioso o
un error confuso) y **fallos espurios** en máquinas o *runners* sin Podman ni los
contenedores locales, esos tests están marcados con `#[ignore]`. Así siguen
compilándose en cada `cargo test` pero no se ejecutan salvo petición explícita.

## Ficheros implicados

- `backend/src/test_support.rs` — módulo compartido (solo bajo `cfg(test)`):
  resolución del socket, helper `podman_client()` y `FixtureContainer` (contenedores
  efímeros de test).
- `backend/src/db.rs` — `apply_schema()`, el esquema compartido por `init_db()` y `test_pool()`.
- `backend/src/containers.rs` — 10 tests de integración (`test_integration_*`).
- `backend/src/stacks.rs` — 2 tests de integración (`test_integration_*`).
- `.justfile` — recipes `test` (unitarios) y `test-integration` (ignored, con preflight de socket).
- `.github/workflows/ci.yml` — guardia `Report ignored integration tests`.

## Cómo funciona

```sh
# Solo unitarios: los 12 de integración aparecen como `ignored`
cargo test          # o: just test

# Solo integración (necesita daemon y contenedores locales)
cargo test -- --ignored   # o: just test-integration
```

**Resolución del socket** (en orden de prioridad):

1. `DOCKER_HOST` (si está definido y no vacío).
2. `$XDG_RUNTIME_DIR/podman/podman.sock` (**solo si el fichero existe**; en un host
   rootful `XDG_RUNTIME_DIR` está definido aunque no haya socket de usuario).
3. `/run/podman/podman.sock` (solo si el fichero existe).
4. `/run/user/1000/podman/podman.sock` (fallback legacy).

**Contenedores de fixture**: los tests ya no asumen contenedores del entorno. Cada test que
necesita uno llama a `FixtureContainer::run("<prefijo>")`, que hace *pull* de una imagen mínima
(`docker.io/library/alpine:3.23`, una sola vez por proceso), crea y arranca un contenedor con un
**nombre único** (`<prefijo>-<pid>-<contador>`), publica un puerto (`80/tcp`) y lo **elimina al
terminar** (`Drop` → `remove_container(force)`). Así la suite es **hermética**: funciona en
cualquier daemon sin contenedores preexistentes y no deja residuos.

**Esquema de test**: `test_pool()` reutiliza `apply_schema()` —el **mismo** esquema que
`init_db()`— de modo que el pool de test no puede divergir del real (antes omitía columnas como
`last_remote_digest`/`last_pulled_digest` y `fetch_containers` devolvía vacío en test).

`just test-integration` aplica la misma cascada como **preflight** y aborta con un
mensaje accionable *antes* de lanzar `cargo` si no hay ningún socket. Si se ejecuta
`cargo test -- --ignored` directamente y no hay daemon, `podman_client()` **paniquea**
con el socket resuelto y la sugerencia de definir `DOCKER_HOST` o arrancar el socket con
`systemctl --user start podman.socket`. No hay *skip* silencioso.

## Requisitos para ejecutarlos

- Daemon Podman (o Docker) accesible desde el socket.
- Acceso a un registry para el **pull inicial** de la imagen de fixture
  (`docker.io/library/alpine:3.23`); queda en caché las siguientes veces.
- Permiso para **crear/arrancar/eliminar** contenedores (los fixtures se limpian solos).

No hacen falta contenedores preexistentes.

Ejemplo apuntando a Docker:

```sh
DOCKER_HOST=unix:///var/run/docker.sock just test-integration
```

## Cómo habilitar/desplegar

No aplica: son tests **locales**, no un servicio desplegable.

En CI **no se ejecutan** porque dependen de contenedores concretos de la máquina.
CI solo comprueba que siguen existiendo exactamente ≥12 tests ignorados mediante el
step informativo+guardia `Report ignored integration tests` (`cargo test -- --ignored --list`);
si alguien elimina un `#[ignore]` y quedan menos de 12, el step falla.

## Decisiones

- **`#[ignore]` en lugar de una feature de Cargo**: idiomático, sin doble
  configuración y los tests siguen compilándose en cada `cargo test`.
- **Helper único compartido** (`test_support.rs`) en vez de duplicar la lógica en
  `containers.rs` y `stacks.rs`.
- **Resolución en cascada** en vez de un UID `1000` hardcodeado, para soportar
  rootless, rootful y `DOCKER_HOST` explícito.
- **Preflight en `just test-integration`** (y no solo el panic de `podman_client()`)
  para fallar rápido y sin ruido cuando no hay daemon.
- **Fixtures efímeros** (`FixtureContainer`) en vez de nombres fijos del entorno: la suite es
  hermética y no depende de contenedores del autor. `Drop` garantiza la limpieza.
- **Esquema compartido** (`apply_schema`) en vez de duplicar el `CREATE TABLE` en `test_pool()`:
  evita el drift de columnas por construcción.

## Follow-ups

- Evaluar **ejecutar los tests de integración en CI**: al ser ahora herméticos, sería posible
  con un runner con Docker; requiere mapear `/var/run/docker.sock` en la resolución del socket
  y asumir el pull de la imagen. Hoy siguen siendo opt-in (guardia `Report ignored integration
  tests` en `ci.yml`).
