# ci/cache Specification

## Purpose
Esta capability define el cacheo de compilación en CI: `Swatinem/rust-cache@v2` para los jobs de test y build, y cache de capas en el build Docker de release. Reduce el tiempo de los pipelines manteniendo los workflows válidos.

## Requirements

### Requirement: CI SHALL cachear la compilación Rust

El workflow `ci.yml` MUST usar `Swatinem/rust-cache@v2` en el job `test`, después de instalar el toolchain y antes de ejecutar `cargo test`, con `workspaces: backend`. El workflow `release.yml` MUST usar el mismo cache en el job `build`.

#### Scenario: Cache en CI

- **GIVEN** el workflow `ci.yml`
- **WHEN** se ejecuta el job `test`
- **THEN** existe un step `Swatinem/rust-cache@v2` con `workspaces: backend`
- **AND** se ejecuta antes de `cargo test`

#### Scenario: Cache en release

- **GIVEN** el workflow `release.yml`
- **WHEN** se ejecuta el job `build`
- **THEN** existe un step `Swatinem/rust-cache@v2` con `workspaces: backend`
- **AND** se ejecuta antes de `cargo build --release`

### Requirement: El build Docker de release SHALL usar cache de capas

El job `docker` de `release.yml` MUST configurar buildx con `docker/setup-buildx-action@v4` antes de `docker/build-push-action@v7`. El step de build MUST declarar `cache-from: type=gha` y `cache-to: type=gha,mode=max`.

#### Scenario: Buildx y cache GHA

- **GIVEN** el workflow `release.yml`
- **WHEN** se ejecuta el job `docker`
- **THEN** existe un step `docker/setup-buildx-action@v4` antes del build
- **AND** el build usa `cache-from: type=gha`
- **AND** el build usa `cache-to: type=gha,mode=max`

### Requirement: Los workflows SHALL seguir siendo válidos

Los workflows modificados MUST pasar `actionlint` sin errores.

#### Scenario: Lint de workflows

- **GIVEN** los workflows modificados
- **WHEN** se ejecuta `actionlint` sobre `.github/workflows/`
- **THEN** no hay errores
