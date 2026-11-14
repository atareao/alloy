# Tasks

## 1. RED — Demostrar el fallo sin fixtures

- [x] 1.1 En un entorno con daemon **sin** los contenedores `alloy`/`oxinbox`, ejecutar
      `just test-integration` y guardar la salida literal: los tests que buscan esos nombres
      **fallan** (`Container 'alloy' should be found`). Evidencia de RED.
- [x] 1.2 Test unitario del helper de nombres (`fixture_name`): dos llamadas dan nombres distintos
      y respetan el prefijo. Debe **fallar** al escribirlo (aún no existe). Evidencia de RED.
- [x] 1.3 RED de `test_pool`: un test que verifica que `test_pool()` tiene las columnas
      `last_remote_digest`/`last_pulled_digest` (o que `fetch_containers` no devuelve vacío por
      error de esquema). Debe **fallar** antes del fix.

## 2. GREEN — Fixtures efímeros

- [x] 2.1 `test_support.rs`: `FixtureContainer` (`run`, `name`, `Drop`) + `ensure_image` + generador
      de nombre único.
- [x] 2.2 `containers.rs` / `stacks.rs`: sustituir los literales `alloy`/`oxinbox`/`preemer-db` por
      el nombre del fixture; actualizar la razón de `#[ignore]`.
- [x] 2.3 `db.rs`: extraer `apply_schema(conn)` y usarla en `init_db()` y `test_pool()`.
- [x] 2.4 Tests unitarios de `fixture_name` y del esquema de `test_pool` en verde.

## 3. REFACTOR y verificación

- [x] 3.1 `just test-integration` **en verde** en un daemon sin contenedores preexistentes de los
      tests; guardar la salida.
- [x] 3.2 Comprobar que **no quedan contenedores de fixture** tras la suite (`podman ps -a`).
- [x] 3.3 `cargo test` (unitarios) verde y los 12 tests siguen `ignored`.
- [x] 3.4 `cd backend && just check` (fmt + clippy `-D warnings`).

## 4. Documentación y cierre

- [x] 4.1 `docs/integration-tests.md`: retirar el follow-up resuelto; documentar la imagen base y el
      pull de la primera ejecución.
- [x] 4.2 Auditar (`rust-reviewer`), abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
