# Proposal

## Why

Los 12 tests de integración del backend no son reproducibles fuera del entorno del autor, por
**dos** causas independientes:

1. Asumen contenedores con nombres concretos del entorno (`alloy`, `oxinbox`, `preemer-db`); en
   cualquier otro entorno —y **también en esta máquina hoy**, donde no existen `alloy` ni
   `oxinbox`— `just test-integration` falla porque los tests dependen de **contenedores
   preexistentes** en vez de provisionar los suyos.
2. `test_pool()` crea la tabla `containers` **sin** las columnas que añaden las migraciones de
   `init_db` (`last_remote_digest`, `last_pulled_digest`). Como `fetch_containers` las consulta,
   el `prepare` falla y devuelve una lista vacía, así que los tests de `fetch_containers` **nunca**
   pasan (verificado en el RED: `test_integration_fetch_containers_returns_list` falla con
   "Should have at least one container" aunque haya contenedores en marcha).

Ambas causas están documentadas/visibles como follow-up en `docs/integration-tests.md`.

## What Changes

- `backend/src/test_support.rs` (solo `#[cfg(test)]`): nuevo helper `FixtureContainer` que
  provisiona un contenedor efímero vía `bollard` (imagen mínima `alpine`, con un puerto
  publicado y un comando que lo mantiene vivo), con **nombre único** y limpieza por `Drop`
  (`remove_container(force)`).
- `backend/src/containers.rs` / `backend/src/stacks.rs`: los tests de integración usan el nombre
  del fixture en vez de los literales `alloy`/`oxinbox`/`preemer-db`; la razón de `#[ignore]`
  deja de citar contenedores concretos.
- `backend/src/db.rs`: se **extrae el esquema** (CREATE TABLE + migraciones) a una función
  compartida `apply_schema()` que usan tanto `init_db` como `test_pool`, eliminando la
  duplicación que causaba el drift de columnas.
- `docs/integration-tests.md`: se retira el follow-up (resuelto) y se documenta el nuevo
  requisito de la imagen base (pull una vez).

Sin cambios en código de aplicación ni en el contrato opt-in (`#[ignore]` + `just test-integration`).

## Capabilities

### New Capabilities

- Ninguna.

### Modified Capabilities

- `backend/integration-tests`: se añade (ADDED) el requisito de **hermeticidad** (los tests deben
  provisionar sus propios contenedores y no depender de los preexistentes), se añade (ADDED) el
  requisito de que **el pool de test use el mismo esquema que producción**, y se **modifica** el
  requisito «Tests de integración opt-in» para que su escenario de éxito no presuponga
  contenedores ajenos.

## Impact

- `backend/src/test_support.rs` (nuevo helper `FixtureContainer`).
- `backend/src/db.rs` (`apply_schema` compartido por `init_db` y `test_pool`).
- `backend/src/containers.rs`, `backend/src/stacks.rs` (tests de integración).
- `docs/integration-tests.md`.
- Requiere **pull de una imagen mínima** en la primera ejecución de `just test-integration`
  (la suite ya requería daemon + red hacia un registry).
