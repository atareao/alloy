# Spec Delta

## Purpose

Garantizar que el build de la imagen Docker sea reproducible en local y en CI, sin que
artefactos locales (`node_modules`) contaminen el contexto de build ni rompan `pnpm`.

## ADDED Requirements

### Requirement: El contexto de build excluye node_modules

`.dockerignore` MUST excluir `frontend/node_modules/` para que `COPY frontend/ ./` no
sobrescriba las dependencias instaladas dentro de la imagen.

#### Scenario: Build local con node_modules presente

- **Given** un `frontend/node_modules` local generado con otra versión de pnpm
- **When** se ejecuta `just build` (podman build)
- **Then** el contexto de build NO incluye `frontend/node_modules`
- **And** `pnpm install --frozen-lockfile` y `pnpm run build` completan sin error

#### Scenario: Build en CI sin node_modules

- **Given** un checkout limpio sin `frontend/node_modules`
- **When** se construye la imagen
- **Then** el build se comporta igual que antes (sin regresión)

### Requirement: El stage de frontend es no-interactivo

El stage `frontend-builder` MUST ejecutar pnpm en modo no-interactivo (`CI=true`) para que
ninguna operación espere confirmación por TTY.

#### Scenario: pnpm sin TTY

- **Given** un build sin terminal interactiva
- **When** pnpm necesita purgar o resolver dependencias
- **Then** no aborta por falta de TTY
