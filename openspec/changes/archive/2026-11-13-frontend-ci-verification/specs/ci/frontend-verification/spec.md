# Spec Delta

## Purpose

Garantizar que todo PR sobre Alloy verifique automáticamente el frontend —tipos, tests, build y
lint— usando el mismo gestor de paquetes y la misma versión mayor de Node que el build de
producción, de forma independiente de la verificación del backend.

## ADDED Requirements

### Requirement: Verificación del frontend en cada PR

El workflow de CI MUST ejecutar un job dedicado al frontend para cada PR dirigido a `main` o
`develop`. El job MUST instalar dependencias con `pnpm install --frozen-lockfile`, MUST fallar si
el lockfile no está sincronizado con `package.json`, y MUST ejecutar en pasos separados:
type-check, tests, build y lint. Un fallo en cualquiera de esos pasos MUST hacer fallar el job y,
por tanto, el PR.

#### Scenario: PR con frontend sano

- **Given** un PR cuyo frontend tiene los tipos correctos, pasa los tests y pasó el lint
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend termina con éxito

#### Scenario: Error de tipos detectado

- **Given** un PR que introduce un error de TypeScript
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de type-check
- **And** el PR no queda en verde

#### Scenario: Test roto detectado

- **Given** un PR que rompe un test de Vitest
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de tests

#### Scenario: Build roto detectado

- **Given** un PR que rompe el build de Vite
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de build

#### Scenario: Lockfile desincronizado

- **Given** un PR que modifica `frontend/package.json` sin regenerar `frontend/pnpm-lock.yaml`
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla en el paso de instalación
- **And** el mensaje de error indica que el lockfile no está al día

### Requirement: Coherencia de toolchain entre CI y build de producción

El job de frontend MUST usar la misma versión mayor de Node que el stage `frontend-builder` del
`Dockerfile`, y MUST obtener la versión de pnpm del campo `packageManager` de
`frontend/package.json` en lugar de declararla por duplicado en el workflow.

#### Scenario: Versión de pnpm de una única fuente

- **Given** el campo `packageManager` de `frontend/package.json`
- **When** el workflow prepara pnpm
- **Then** la versión instalada deriva de ese campo
- **And** el workflow no declara una versión de pnpm propia

#### Scenario: Node alineado con el Dockerfile

- **Given** que el `Dockerfile` construye el frontend sobre `node:23-alpine`
- **When** el job de frontend configura Node
- **Then** usa la versión mayor 23

### Requirement: Independencia entre los jobs de frontend y backend

El job de frontend MUST ser independiente del job de backend. La ausencia de un socket Podman en
el runner MUST NOT impedir la verificación del frontend, y un fallo del frontend MUST NOT omitir
la verificación del backend.

#### Scenario: Ambos jobs se ejecutan en un runner sin Podman

- **Given** un runner sin socket Podman y sin contenedores en ejecución
- **When** se ejecuta el workflow de CI
- **Then** el job de backend y el job de frontend se ejecutan ambos
- **And** el job de frontend no depende del resultado del job de backend

#### Scenario: Fallo de frontend no oculta el backend

- **Given** un PR con el frontend roto y el backend sano
- **When** se ejecuta el workflow de CI
- **Then** el job de frontend falla
- **And** el job de backend termina con éxito
