# ci/rust-lockfile Specification

## Purpose

Definir el contrato del lockfile de Rust de Alloy: el `backend/Cargo.lock` versionado MUST estar
sincronizado con `backend/Cargo.toml`, la verificación de CI y la build de release MUST detectar y
bloquear cualquier desincronización, y el guard local MUST impedir commitear el drift, sin alterar
por el camino la versión de ninguna dependencia.

## ADDED Requirements

### Requirement: El lockfile commiteado está sincronizado con el manifiesto

El `backend/Cargo.lock` versionado MUST declarar la misma versión para el paquete raíz `alloy` que
`backend/Cargo.toml`, y MUST NOT contener entradas obsoletas ni ausencias que obliguen a `cargo` a
reescribirlo.

#### Scenario: Versiones del paquete raíz coinciden

- **Given** el repositorio recién clonado en cualquier rama de trabajo
- **When** se lee la versión de `alloy` en `backend/Cargo.lock` y en `backend/Cargo.toml`
- **Then** ambas coinciden
- **And** `cargo metadata --manifest-path backend/Cargo.toml --locked` termina con éxito sin reescribir el lock

#### Scenario: Sincronizar la versión no toca las dependencias

- **Given** un `backend/Cargo.toml` con la versión raíz actualizada y el lock desincronizado
- **When** se ejecuta `cargo update -w --manifest-path backend/Cargo.toml`
- **Then** cambia únicamente la versión del paquete raíz `alloy` en el lock
- **And** la versión de ninguna dependencia (`serde`, `axum`, `bollard`, …) cambia

### Requirement: CI y la build de release usan `--locked`

El job de backend de CI y la build del artefacto de release MUST ejecutar los comandos cargo que
resuelven dependencias en modo `--locked`, de modo que un lock desincronizado haga fallar el job (y
el PR) en lugar de pasar oculto, y el binario publicado se construya siempre desde el lock
commiteado.

#### Scenario: Lock desincronizado rompe CI

- **Given** un PR que modifica la versión de `backend/Cargo.toml` sin actualizar `backend/Cargo.lock`
- **When** se ejecuta el job de backend de CI
- **Then** el comando con `--locked` falla indicando que el lock necesita actualizarse
- **And** el job (y el PR) no quedan en verde

#### Scenario: Lock sincronizado pasa CI

- **Given** un PR con `backend/Cargo.lock` sincronizado con `backend/Cargo.toml`
- **When** se ejecuta el job de backend de CI
- **Then** los comandos con `--locked` terminan con éxito

#### Scenario: La build de release usa el lock commiteado

- **Given** el workflow `release.yml` construyendo el artefacto de una versión
- **When** ejecuta `cargo build --release --locked`
- **Then** la build falla si el lock no está sincronizado, en lugar de reescribirlo en silencio

### Requirement: El guard local detecta el drift antes de commitear

El pre-commit del repositorio (`just check`) MUST ejecutar `cargo clippy` en modo `--locked`, de forma
que un desarrollador descubra la desincronización del lock antes de commitear, no solo en CI.

#### Scenario: `just check` falla con el lock desincronizado

- **Given** un `backend/Cargo.toml` con la versión cambiada y el lock sin actualizar
- **When** se ejecuta `just check`
- **Then** el paso de `clippy --locked` falla indicando que el lock necesita actualizarse
