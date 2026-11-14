# ci/release-automation Specification

## ADDED Requirements

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
