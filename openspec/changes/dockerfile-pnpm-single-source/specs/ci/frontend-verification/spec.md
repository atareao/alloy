# ci/frontend-verification Specification

## ADDED Requirements

### Requirement: El Dockerfile deriva la versión de pnpm de `packageManager`

El stage `frontend-builder` del `Dockerfile` MUST obtener la versión de pnpm del campo
`packageManager` de `frontend/package.json`, sin declararla por duplicado en ninguna línea de
código, y el workflow de CI MUST fallar si el código del `Dockerfile` fija una versión literal de
pnpm (directamente, vía una variable, o parcialmente, p. ej. `pnpm@11.21`) o si deja de derivarla de
`packageManager`.

#### Scenario: El Dockerfile instala la versión declarada en packageManager

- **Given** `frontend/package.json` con `packageManager` = `pnpm@X`
- **When** el stage `frontend-builder` instala pnpm
- **Then** instala la versión derivada de `packageManager`
- **And** ninguna línea de código del `Dockerfile` contiene una versión literal de pnpm (los comentarios no cuentan)

#### Scenario: Deriva entre el Dockerfile y packageManager

- **Given** un `Dockerfile` cuyo código fija una versión literal de pnpm (directamente, vía una
  variable, o parcialmente) o que ya no lee `packageManager`
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de comprobación de consistencia de pnpm

#### Scenario: Los comentarios no cuentan como deriva

- **Given** un `Dockerfile` que conserva el comentario que menciona `packageManager` pero cuyo
  código ya no lo consume
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de comprobación de consistencia de pnpm
