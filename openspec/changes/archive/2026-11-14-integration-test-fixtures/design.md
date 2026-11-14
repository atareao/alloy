# Design

## Contexto

12 tests de integración (`containers.rs`: 10, `stacks.rs`: 2) hablan con el socket del daemon vía
`test_support::podman_client()`. Varios asumen contenedores con nombre fijo:

```
find(|c| c.name == "alloy").expect("Container 'alloy' should be running");
find_container_by_name(&docker, "oxinbox").await.expect(...);
find_container_by_name(&docker, "preemer-db").await.expect(...);
```

La razón de `#[ignore]` cita esos nombres. `fetch_containers` funciona con cualquier contenedor,
así que el acoplamiento es **solo el nombre**.

## Decisiones

### D1 — Fixtures efímeros vía `bollard` (enfoque A)

Cada test provisiona sus propios contenedores y los destruye al terminar. Ventajas frente a
parametrizar por variable de entorno (B) o descubrir contenedores existentes (C):

- **Hermético**: funciona en cualquier daemon sin depender de contenedores del entorno.
- **Sin skip silencioso**: (C) reintroduciría el "no hay contenedores → skip" que #102 quiso evitar.
- **Sin configuración**: (B) requiere que el usuario defina los nombres; no arregla el fallo por defecto.

### D2 — Imagen base mínima y comando que la mantiene viva

Se usa `docker.io/library/alpine:3.23` (misma base que el runtime del `Dockerfile`, ~8 MB) con
`cmd = ["sleep", "600"]` (busybox incluye `sleep`). Para que `fetch_containers` produzca `ports`
no vacío, el fixture **publica un puerto** (`ExposedPorts` 80/tcp + `HostConfig.port_bindings`
80/tcp → puerto efímero del host). Un contenedor `alpine` con un puerto publicado satisface las
aserciones de `test_integration_fetch_containers_structure` (`state == "running"`, `ports` no vacío).

El pull se hace **una sola vez por proceso de test** (`ensure_image`, guardado con `OnceCell`);
si la imagen ya está en la caché local, se omite.

### D3 — Nombres únicos y limpieza RAII

- Nombre: `alloy-it-<pid>-<contador_atómico>`. El `pid` aísla procesos concurrentes; el contador
  atómico global evita colisiones entre tests que corren en paralelo (por defecto `cargo test` usa
  varios threads).
- `FixtureContainer` implementa `Drop`: lanza el `remove_container(force=true)` en un hilo con su
  propio runtime (no se puede `block_on` dentro del runtime de `#[tokio::test]`). Verificado por el
  escenario «no queda ningún contenedor de fixture».

### D4 — Estabilización tras el arranque

Tras `start_container`, el helper espera (con reintentos y timeout) a que el contenedor aparezca
en `list_containers` antes de devolver el fixture, para evitar carreras con el registro del daemon.

### D5 — Alcance

- No se toca `resolve_socket` ni el preflight de `just test-integration`.
- **No se ejecutan los tests en CI**: siguen requiriendo un daemon. La guardia `Report ignored
  integration tests` de `ci.yml` (cuenta ≥12 `ignored`) se mantiene; habilitarlos en CI es un
  follow-up aparte (requiere Docker en el runner y mapear `/var/run/docker.sock`).

### D6 — Esquema de test compartido con producción

`test_pool()` duplicaba el `CREATE TABLE` y **omitía** las columnas añadidas por migraciones
(`last_remote_digest`, `last_pulled_digest`). `fetch_containers` consulta esas columnas, así que en
test el `prepare` fallaba y devolvía `Vec::new()` — de ahí que los tests de `fetch_containers`
nunca pasaran. Se extrae el esquema (CREATE TABLE + migraciones idempotentes) a
`apply_schema(conn: &Connection)`, invocada por `init_db()` (tras los `PRAGMA`) y por `test_pool()`.
Así el drift de columnas es imposible por construcción. Los `PRAGMA` (`journal_mode=WAL`,
`foreign_keys=ON`) se mantienen solo en `init_db()` para no cambiar la semántica del pool de test.

## Fuera de alcance

- Reducir el número de tests. Los duplicados (`..._oxinbox`, `..._postgres`) se conservan como
  cobertura de "varios fixtures con nombres distintos coexisten".
