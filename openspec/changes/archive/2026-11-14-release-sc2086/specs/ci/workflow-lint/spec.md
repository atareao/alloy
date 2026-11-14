# ci/workflow-lint Specification

## Purpose

Garantizar que los scripts embebidos en los workflows de GitHub Actions de Alloy pasan `actionlint`
sin avisos —en particular las escrituras a los ficheros de entorno de GitHub— y que CI lo verifica
en cada PR, de modo que un aviso nuevo no pase desapercibido.

## ADDED Requirements

### Requirement: Los scripts de los workflows pasan `actionlint`

Los scripts embebidos en los pasos `run:` de los workflows MUST pasar `actionlint` sin avisos. En
particular, las escrituras a los ficheros de entorno de GitHub (`$GITHUB_ENV`, `$GITHUB_OUTPUT`,
`$GITHUB_PATH`) MUST ir entre comillas dobles.

El workflow de CI MUST ejecutar `actionlint` (sin argumentos, que autodescubre `.github/workflows/`)
sobre cada PR a `main` y `develop`, y MUST fallar ante cualquier aviso. La herramienta MUST disponer
de `shellcheck`, que es quien aporta los diagnósticos `SC####`; sin él, `actionlint` no los reporta.

#### Scenario: El paso *Prepare archive* escribe en `$GITHUB_ENV` comillado

- **Given** el paso *Prepare archive* de `release.yml` que exporta `ARTIFACT`
- **When** se ejecuta `actionlint` sobre los workflows
- **Then** no se reporta ningún `SC2086`
- **And** el paso escribe en `"$GITHUB_ENV"` sin cambiar el valor exportado

#### Scenario: Un workflow con un aviso de `actionlint` rompe CI

- **Given** un PR a `main` o `develop` que introduce en un workflow un script con un aviso de `actionlint`
- **When** se ejecuta el workflow de CI
- **Then** el job de lint de workflows falla
- **And** el PR no queda en verde

#### Scenario: Los workflows del repositorio pasan `actionlint`

- **Given** los workflows de `.github/workflows/`
- **When** CI ejecuta `actionlint` con `shellcheck` disponible
- **Then** el comando termina con éxito y sin avisos
