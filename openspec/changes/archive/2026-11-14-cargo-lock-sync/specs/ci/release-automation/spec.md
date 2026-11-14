# ci/release-automation Specification

## MODIFIED Requirements

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
