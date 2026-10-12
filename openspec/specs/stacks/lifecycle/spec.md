# stacks/lifecycle Specification

## Purpose
Definir la gestión de stacks Docker Compose (actualización y bajada) **sin depender del CLI
`docker`**, usando la API de Docker (Bollard), y garantizar que cualquier fallo sea visible
al usuario a través de la respuesta de la API.

## Requirements

### Requirement: Actualización de stack vía API

`POST /api/stacks/{project}/update` MUST actualizar cada servicio del proyecto haciendo
`pull` de la imagen y recreando el contenedor del servicio vía la API de Docker. MUST NOT
invocar el CLI `docker`. El resultado por servicio MUST indicar `status:"ok"` con la
duración, o `status:"error"` con la causa.

#### Scenario: Update de stack con éxito

- **Given** un proyecto de compose con servicios corriendo
- **When** se llama a `POST /api/stacks/{project}/update`
- **Then** por cada servicio se hace pull de la imagen y se recrea el contenedor vía API
- **And** el resultado reporta `status:"ok"` con `duration_ms`
- **And** se persiste el config digest del contenedor recreado como `last_remote_digest`
- **And** se emite notificación por servicio

#### Scenario: Fallo de pull o recreate

- **Given** un servicio cuya imagen no puede descargarse o recrearse
- **When** se llama al update de stack
- **Then** el resultado de ese servicio reporta `status:"error"` con la causa
- **And** el resto de servicios continúa procesándose

#### Scenario: Proyecto inexistente

- **Given** un `project` sin contenedores con la label de compose
- **When** se llama al update
- **Then** la respuesta es `404 NotFound`

### Requirement: Bajada de stack vía API

`POST /api/stacks/{project}/down` MUST detener y eliminar los contenedores del proyecto vía
la API de Docker, sin invocar el CLI `docker`.

#### Scenario: Down de stack con éxito

- **Given** un proyecto de compose con servicios corriendo
- **When** se llama a `POST /api/stacks/{project}/down`
- **Then** cada contenedor del proyecto se detiene y se elimina vía API
- **And** se eliminan las redes del proyecto (label `com.docker.compose.project`)
- **And** NO se eliminan los volúmenes con nombre (los datos se preservan)
- **And** la respuesta reporta `status:"removed"`

#### Scenario: Red del proyecto en uso por un contenedor externo

- **Given** una red del proyecto compartida con un contenedor ajeno al proyecto
- **When** se intenta `remove_network`
- **Then** el fallo se registra como `WARN` y el down continúa
- **And** la respuesta no aborta por esa red no eliminada

#### Scenario: Proyecto inexistente

- **Given** un `project` sin contenedores
- **When** se llama al down
- **Then** la respuesta es `404 NotFound`

### Requirement: Descubrimiento de stacks sin CLI

El listado de stacks MUST derivarse exclusivamente de las labels de Docker
(`com.docker.compose.project`/`service`). MUST NOT depender de `docker compose ls`.

#### Scenario: Listado por labels

- **Given** contenedores con labels de compose
- **When** se llama a `GET /api/stacks`
- **Then** se agrupan por `project` y `service` sin ejecutar ningún proceso externo

#### Scenario: Sin fallback CLI

- **Given** que el binario `docker` no existe
- **When** se opera cualquier ruta de stacks
- **Then** no hay errores de proceso no encontrado ni resultados vacíos por CLI ausente

### Requirement: Aplicación de política PullRestartStack vía API

La rama `PullRestartStack` de `apply_single_policy` (ruta de aplicar políticas/check-all)
MUST actualizar el servicio vía API con la misma semántica que el worker del scheduler
(`pull_image` + `recreate_container`), sin CLI, y reportar éxito/error de forma observable.

#### Scenario: Política aplicada con éxito

- **Given** un contenedor con política `PullRestartStack` y update disponible
- **When** se aplica la política desde la ruta API
- **Then** se hace pull y recreate vía API
- **And** se notifica, se marca `has_update=false` y se persiste `last_remote_digest`

#### Scenario: Fallo de la aplicación

- **Given** un fallo de pull o recreate
- **When** se aplica la política
- **Then** se registra `ERROR` con la causa y el contenedor no se marca como actualizado
