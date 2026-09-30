# updates/digest-pinned-refs Specification

## Purpose
Define cómo Alloy interpreta referencias de imagen fijadas por digest (`repo:tag@sha256:...`),
cómo comprueba actualizaciones por tag y cuándo avanza el digest que representa la imagen
en ejecución, de modo que la detección de updates nunca se desactive por un pin generado
por el propio Alloy.

## Requirements

### Requirement: parse_image_ref conserva tag y digest

`parse_image_ref` MUST conservar el tag anterior a `@` y exponer el digest en un campo
propio, sin sustituir el tag por el literal `"digest"`.

#### Scenario: Referencia pinneada con tag

- **Given** la referencia `gitea/gitea:1.27.3@sha256:87a6...`
- **When** se parsea
- **Then** `registry=docker.io`, `repo=gitea/gitea`, `tag=1.27.3`
- **And** `digest=Some("sha256:87a6...")` y `digest_only=false`

#### Scenario: Referencia pinneada sin tag

- **Given** la referencia `nginx@sha256:abc...`
- **When** se parsea
- **Then** `tag=latest` y `digest=Some("sha256:abc...")`

#### Scenario: Digest puro

- **Given** la referencia `sha256:abc...`
- **When** se parsea
- **Then** `digest_only=true` (comportamiento actual preservado)

### Requirement: El check de update consulta por el tag

`check_remote_digest_with_docker` MUST consultar el registro usando `repo:tag`, nunca la
referencia con `@digest` ni el literal `"digest"`.

#### Scenario: Imagen pinneada por Alloy

- **Given** un contenedor con `Config.Image=gitea/gitea:1.27.3@sha256:87a6...`
- **When** se ejecuta el check de update
- **Then** consulta `gitea/gitea:1.27.3`, no `.../manifests/digest`
- **And** no produce HTTP 404
- **And** si el tag apunta a un config digest distinto al `image_id`, `has_update=true`

#### Scenario: Imagen normal sin digest

- **Given** un contenedor con `Config.Image=nginx:alpine`
- **When** se ejecuta el check
- **Then** no hay petición extra y el comportamiento es idéntico al actual

### Requirement: Config.Image se mantiene limpio al recrear

Tras un update, el contenedor recreado MUST referenciar `repo:tag`, sin `@sha256`.

#### Scenario: Recreate limpia el pin

- **Given** un contenedor con `Config.Image=repo:tag@sha256:viejo` y update disponible
- **When** se aplica la política `PullRestart`
- **Then** se hace pull por digest exacto y se retagüea `repo:tag`
- **And** el nuevo `Config.Image` es `repo:tag`

### Requirement: El digest en uso solo avanza al recrear

`last_remote_digest` representa el config digest que el contenedor ejecuta y MUST avanzar
SOLO tras un recreate exitoso (o una actualización de stack). Las políticas sin recreate
no lo avanzan.

#### Scenario: Seed inicial

- **Given** un contenedor sin `last_remote_digest`
- **And** `image_id=sha256:aaa`
- **When** el remoto (por tag) resuelve `config_digest=sha256:bbb`
- **Then** `has_update=true`
- **And** tras recreate exitoso se persiste `sha256:bbb`

#### Scenario: Sin update no avanza

- **Given** `last_remote_digest=sha256:aaa`
- **And** el remoto resuelve `sha256:aaa`
- **When** finaliza el check
- **Then** `has_update=false` y `last_remote_digest` sigue `sha256:aaa`

#### Scenario: Política None no avanza

- **Given** `last_remote_digest=sha256:aaa` y remoto `sha256:bbb` (hay update)
- **When** la política es `None`
- **Then** `last_remote_digest` NO cambia (el contenedor sigue viejo)

#### Scenario: Política Pull no avanza

- **Given** update disponible y política `Pull` (sin recreate)
- **When** se aplica
- **Then** se hace pull pero `last_remote_digest` NO avanza

#### Scenario: PullRestartStack persiste el digest

- **Given** update disponible en un contenedor de stack
- **When** `docker compose pull && up -d` termina con éxito
- **Then** se resuelve el config digest local del servicio
- **And** se persiste como `last_remote_digest`
