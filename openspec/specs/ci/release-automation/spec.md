# ci/release-automation Specification

## Purpose
Garantizar que publicar una versión de Alloy (bump de versión, tag, GitHub Release e imagen
Docker) sea fiable al integrar cambios en `main`, y que `develop` no se desincronice.

## Requirements

### Requirement: El push del tag dispara el workflow de Release

`release-prepare.yml` MUST autenticar sus operaciones de git con un token distinto del
`GITHUB_TOKEN` por defecto (secreto `GH_PAT`), de modo que el push del tag `vX` genere
**por sí mismo** un run de `release.yml`. MUST NOT existir un paso `gh workflow run` para
disparar el release.

#### Scenario: Cambios nuevos llegan a main

- **Given** un merge a `main` con commits que no son `chore: release`
- **When** corre `release-prepare.yml`
- **Then** crea el commit `chore: release vX` y el tag `vX`, y los empuja con `GH_PAT`
- **And** el push del tag dispara un run de `release.yml` para `vX`
- **And** no hay ningún paso `gh workflow run` en el workflow

#### Scenario: El propio commit de release no re-dispara el workflow

- **Given** que `release-prepare.yml` empuja el commit `chore: release vX` a `main`
- **When** el push dispara el workflow de nuevo
- **Then** el job se omite por la guarda de `head_commit.message`

### Requirement: La versión se calcula con vampus y el changelog con git-cliff

El tipo de bump MUST derivarse de los commits (`!: ` ⇒ major, `^feat` ⇒ minor, resto ⇒
patch). La versión MUST actualizarse con `vampus` en `.vampus.yml` y `backend/Cargo.toml`,
`backend/Cargo.lock` MUST sincronizarse con la versión nueva (`cargo update -w`) antes de crear el
commit de release, y el `CHANGELOG.md` MUST generarse con `git-cliff` para el tag nuevo.

#### Scenario: Commits con feat producen un bump minor

- **Given** commits `✨ feat(...)` desde el último tag
- **When** corre el cálculo de bump
- **Then** el tipo es `--minor` y `vampus` sube la versión minor

#### Scenario: El commit de release deja el lockfile sincronizado

- **Given** un push a `main` que dispara `release-prepare.yml`
- **When** el workflow bumpea la versión con vampus
- **Then** sincroniza `backend/Cargo.lock` con la versión nueva (`cargo update -w`)
- **And** el commit `chore: release vX` incluye el lock sincronizado, de modo que la guarda de
  `--locked` de CI sigue pasando

### Requirement: develop se sincroniza con main tras el release

Tras crear el tag, el workflow MUST sincronizar `develop` con `main` (merge de `main` en
`develop`), de modo que la versión y el commit de release no queden solo en `main`.

#### Scenario: develop no queda desincronizado

- **Given** un release recién publicado en `main`
- **When** termina `release-prepare.yml`
- **Then** `develop` contiene el commit `chore: release vX` y `.vampus.yml` con la versión nueva
- **And** el paso de sincronización no depende de que un paso de dispatch previo haya tenido éxito

### Requirement: El secreto GH_PAT está documentado

El workflow MUST depender de un secreto `GH_PAT` con permiso para empujar y disparar
workflows. Su existencia y renovación MUST estar documentadas.

#### Scenario: PAT ausente o expirado

- **Given** que `GH_PAT` no existe o está expirado
- **When** corre `release-prepare.yml`
- **Then** el fallo se registra de forma explícita (no como un 401 críptico en un paso de
  dispatch) y la documentación indica cómo renovarlo

### Requirement: El workflow de Release no contiene ramas de artefacto muertas

`release.yml` MUST publicar exactamente los artefactos que produce su matriz de build: MUST NOT
contener ramas condicionales ni referencias (pasos, `files:`, expresiones) a formatos de archivo que
ningún target de la matriz use. Si un formato deja de producirse, sus referencias MUST eliminarse en
el mismo cambio.

#### Scenario: La preparación del archivo coincide con la matriz

- **Given** la matriz de build de `release.yml` (targets Linux `x86_64`/`aarch64` con `tar.gz`)
- **When** un target prepara su artefacto
- **Then** produce el formato que declara la matriz, sin ramas alternativas inalcanzables
- **And** no queda ninguna referencia a `7z` ni a `.zip` en el workflow

#### Scenario: El release publica solo artefactos producidos

- **Given** el job `release`
- **When** adjunta ficheros al GitHub Release y genera la tabla de descargas
- **Then** los patrones de `files:` y el parseo de nombres corresponden a los formatos realmente
  producidos (`*.tar.gz`), sin patrones de formatos que no se generan
