# Digest Comparison — Update Detection Logic

## Purpose

Centralizar y unificar la lógica de comparación de digests para determinar si un contenedor necesita una actualización, eliminando falsos positivos en registros no-DockerHub donde el `image_id` de Docker no coincide con el `config_digest` del registro tras un recreate con `image@manifest_digest`.

## ADDED Requirements

### Requirement: needs_update function

**Given** un contenedor con `last_remote_digest` en BD  
**When** se compara con el `config_digest` remoto  
**Then** `needs_update()` retorna `false` si los short digests (12 chars) son iguales  
**And** retorna `true` si son diferentes  
**And** retorna `true` si `local_digest` está vacío

#### Scenario: DockerHub image, no update needed

**Given** un contenedor con imagen `nginx:alpine` de DockerHub  
**And** `last_remote_digest` en BD = `sha256:abc123def456`  
**When** se obtiene `config_digest` del registro = `sha256:abc123def456`  
**Then** `needs_update()` retorna `false`  
**And** no se ejecuta pull ni recreate  
**And** no se registra entrada en el historial

#### Scenario: DockerHub image, update available

**Given** un contenedor con imagen `nginx:alpine` de DockerHub  
**And** `last_remote_digest` en BD = `sha256:abc123def456`  
**When** se obtiene `config_digest` del registro = `sha256:789012ghi345`  
**Then** `needs_update()` retorna `true`  
**And** se ejecuta pull + recreate  
**And** tras el éxito, se persiste `sha256:789012ghi345` como `last_remote_digest`

#### Scenario: Non-DockerHub registry, no update needed

**Given** un contenedor con imagen `public.ecr.aws/zinclabs/openobserve:latest`  
**And** `last_remote_digest` en BD = `sha256:890f25d31020` (config_digest del registry)  
**And** `image_id` de Docker = `sha256:600f92cafd2a` (manifest digest, diferente)  
**When** se obtiene `config_digest` del registro = `sha256:890f25d31020`  
**Then** `needs_update()` retorna `false` (usa `last_remote_digest`, no `image_id`)  
**And** no se ejecuta pull ni recreate

#### Scenario: Non-DockerHub registry, real update available

**Given** un contenedor con imagen `ghcr.io/pocket-id/pocket-id:latest`  
**And** `last_remote_digest` en BD = `sha256:be4e53f22192`  
**When** se obtiene `config_digest` del registro = `sha256:a3f3a5ce25ce`  
**Then** `needs_update()` retorna `true`  
**And** se ejecuta pull + recreate  
**And** tras el éxito, se persiste `sha256:a3f3a5ce25ce` como `last_remote_digest`

#### Scenario: First run, no last_remote_digest in DB

**Given** un contenedor sin `last_remote_digest` en BD (primera ejecución)  
**And** `image_id` de Docker = `sha256:abc123def456`  
**When** se obtiene `config_digest` del registro = `sha256:abc123def456`  
**Then** `needs_update()` usa `image_id` como fallback  
**And** retorna `false` (mismos digests)  
**And** se persiste `last_remote_digest = sha256:abc123def456`

#### Scenario: Empty local digest

**Given** un contenedor sin `last_remote_digest` en BD  
**And** `image_id` vacío (contenedor no existe localmente)  
**When** se llama a `needs_update()`  
**Then** retorna `true` (asume que necesita pull)

#### Scenario: Registry unreachable

**Given** un contenedor con `last_remote_digest` en BD  
**When** el registro no responde  
**Then** `needs_update()` no se evalúa  
**And** el handler retorna error sin modificar `last_remote_digest`

### Requirement: Persist last_remote_digest after successful update

**Given** un update exitoso via cualquier pathway (API single, check-all, scheduler)  
**When** el handler completa  
**Then** se persiste el `config_digest` remoto como `last_remote_digest` en SQLite  
**And** el siguiente ciclo de check detecta que no hay update

#### Scenario: update_container_h persists last_remote_digest

**Given** un update exitoso via `POST /api/update/{name}`  
**When** el handler `update_container_h` completa  
**Then** se llama a `update_container_last_remote_digest()` con el `config_digest` remoto  
**And** el siguiente ciclo de check detecta que no hay update

#### Scenario: check_and_apply_all uses last_remote_digest

**Given** un contenedor con `last_remote_digest` en BD  
**When** `check_and_apply_all` evalúa si necesita update  
**Then** usa `last_remote_digest` (no `image_id`) para la comparación  
**And** no detecta falso positivo aunque `image_id` sea un manifest digest