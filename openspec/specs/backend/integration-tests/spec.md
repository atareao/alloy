# backend/integration-tests Specification

## Purpose
Definir el contrato de la suite de tests del backend: los tests unitarios MUST poder
ejecutarse en cualquier entorno sin daemon de contenedores, y los tests que necesitan
Podman/Docker MUST ser explícitamente opt-in, de forma que nunca produzcan falsos verdes
ni fallos espurios.

## Requirements

### Requirement: `cargo test` sin daemon de contenedores

`cargo test` en `backend/` MUST completar con éxito (exit 0) en un entorno sin socket de
Podman ni de Docker, y MUST reportar como `ignored` todos los tests que requieren daemon.
Ningún test que requiera daemon MUST contarse como `passed` cuando no se ha ejecutado.

#### Scenario: Máquina sin Podman

- **Given** un entorno sin `DOCKER_HOST` y sin socket de Podman en `XDG_RUNTIME_DIR` ni en `/run/podman`
- **When** se ejecuta `cargo test` en `backend/`
- **Then** la suite termina con exit 0
- **And** el resumen no muestra fallos
- **And** los 12 tests de integración aparecen como `ignored`

#### Scenario: Ejecución por defecto no toca el daemon

- **Given** un entorno con Podman en marcha
- **When** se ejecuta `cargo test` en `backend/`
- **Then** los tests marcados como integración no se ejecutan

### Requirement: Tests de integración opt-in

Los tests que interactúan con el daemon de contenedores MUST estar marcados con
`#[ignore]` y una razón legible, y MUST poder ejecutarse bajo demanda mediante
`cargo test -- --ignored` (expuesto como `just test-integration`).

#### Scenario: Ejecución explícita con daemon disponible

- **Given** un daemon de Podman accesible y los contenedores que los tests esperan
- **When** se ejecuta `just test-integration`
- **Then** los tests de integración se ejecutan y terminan con éxito

#### Scenario: Daemon no disponible en ejecución explícita

- **Given** un entorno sin socket de Podman accesible
- **When** se ejecuta `just test-integration`
- **Then** el comando aborta antes de lanzar cargo
- **And** muestra un mensaje que indica cómo definir `DOCKER_HOST` o arrancar el socket

### Requirement: Resolución única del socket

La localización del socket MUST resolverse en un único punto compartido por todos los
tests de integración, con este orden de prioridad: `DOCKER_HOST`, socket de usuario en
`XDG_RUNTIME_DIR` (solo si existe), socket del sistema en `/run/podman/podman.sock`
(solo si existe), y como último recurso `/run/user/1000/podman/podman.sock`. De este modo
un host rootful con `XDG_RUNTIME_DIR` definido pero sin socket de usuario MUST caer al
socket del sistema en vez de devolver una ruta inexistente. Ningún módulo de test MUST
duplicar esa lógica ni asumir un UID concreto.

#### Scenario: `DOCKER_HOST` tiene prioridad

- **Given** `DOCKER_HOST` definido con un valor arbitrario
- **When** un test de integración resuelve el socket
- **Then** se usa exactamente el valor de `DOCKER_HOST`

#### Scenario: Sin `DOCKER_HOST` con socket de usuario presente

- **Given** `XDG_RUNTIME_DIR=/run/user/1234`, sin `DOCKER_HOST`, y existe `/run/user/1234/podman/podman.sock`
- **When** un test de integración resuelve el socket
- **Then** se usa `/run/user/1234/podman/podman.sock`

#### Scenario: Socket de usuario ausente en host rootful

- **Given** `XDG_RUNTIME_DIR=/run/user/1234`, sin `DOCKER_HOST`, sin socket en ese directorio, y existe `/run/podman/podman.sock`
- **When** un test de integración resuelve el socket
- **Then** se usa `/run/podman/podman.sock`
