# backend/integration-tests Specification

## ADDED Requirements

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

## MODIFIED Requirements

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
