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

- **Given** un daemon de contenedores accesible, sin contenedores preexistentes que los tests necesiten
- **When** se ejecuta `just test-integration`
- **Then** los tests de integración se ejecutan, provisionan sus propios fixtures y terminan con éxito

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

### Requirement: Tests de integración herméticos

Los tests de integración que necesitan contenedores MUST provisionar sus **propios** contenedores
efímeros (fixtures) y MUST NOT depender de contenedores preexistentes en el entorno. Cada fixture
MUST tener un nombre único (no colisionable entre tests que corren en paralelo) y MUST ser
eliminado al terminar el test, de modo que la suite no deje contenedores residuales.

#### Scenario: Suite en un daemon sin contenedores preexistentes

- **Given** un daemon de contenedores accesible **sin ningún contenedor** creado por el usuario
- **When** se ejecuta `just test-integration`
- **Then** todos los tests de integración terminan con éxito

#### Scenario: Los fixtures se limpian al terminar

- **Given** una ejecución de los tests de integración que provisiona fixtures
- **When** la suite termina
- **Then** no queda ningún contenedor de fixture en el daemon

#### Scenario: Los nombres de fixture no colisionan

- **Given** dos tests de integración que se ejecutan en paralelo
- **When** cada uno provisiona su fixture
- **Then** cada fixture tiene un nombre distinto y ambos coexisten sin error

### Requirement: El pool de test usa el mismo esquema que producción

`test_pool()` MUST crear un esquema **idéntico** al que produce `init_db()` —tablas y columnas
incluidas las añadidas por migraciones—, de modo que el código bajo test no falle por columnas
ausentes. Para garantizarlo, la creación del esquema MUST estar en un único punto compartido por
`init_db()` y `test_pool()`, sin definiciones duplicadas que puedan divergir.

#### Scenario: `fetch_containers` funciona con el pool de test

- **Given** el pool creado por `test_pool()`
- **When** se llama a `fetch_containers` con contenedores presentes en el daemon
- **Then** no falla por columnas ausentes y devuelve la lista de contenedores

#### Scenario: El esquema de test no diverge del real

- **Given** una columna que `init_db()` añade a `containers` (por migración)
- **When** `test_pool()` crea su esquema
- **Then** esa columna también existe en el esquema de test
