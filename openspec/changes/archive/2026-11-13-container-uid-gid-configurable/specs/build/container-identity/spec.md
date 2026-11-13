# Spec Delta

## Purpose

La imagen Docker de Alloy debe poder arrancar con una identidad de usuario
(UID/GID) ajustable al host, sin dejar de correr sin privilegios ni perder la
compatibilidad con el despliegue por defecto.

## ADDED Requirements

### Requirement: La identidad del proceso es configurable en build

El stage de runtime MUST aceptar los `ARG UID` y `ARG GID` (por defecto `1000`)
y crear el usuario `app` con esa identidad, de modo que la imagen pueda
construirse con el UID/GID de cualquier host.

#### Scenario: Build con la identidad del host

- **Given** un host cuya cuenta es UID 1002 / GID 1002
- **When** se construye la imagen con `--build-arg UID=1002 --build-arg GID=1002`
- **Then** dentro de la imagen `id app` reporta `uid=1002(app) gid=1002`

#### Scenario: Build por defecto

- **Given** un build sin `--build-arg`
- **When** se construye la imagen
- **Then** el usuario `app` queda con UID 1000 y GID 1000 (comportamiento previo)

### Requirement: La identidad del proceso es configurable en runtime

El contenedor MUST leer las variables de entorno `PUID` y `PGID` al arrancar y,
si se indican, reajustar el usuario `app` a esa identidad y cederle la propiedad
de `/app/data` antes de iniciar la aplicación.

#### Scenario: Ajuste por PUID/PGID

- **Given** una imagen construida con los defaults (UID/GID 1000)
- **When** se ejecuta con `PUID=1002 PGID=1002` montando un volumen `data/`
  propiedad del host 1002
- **Then** la aplicación arranca y crea/abre `data/alloy.db` sin error

#### Scenario: Sin variables de identidad

- **Given** una ejecución sin `PUID` ni `PGID`
- **When** arranca el contenedor
- **Then** se usa la identidad por defecto (1000/1000) y el arranque no cambia
  respecto al comportamiento anterior

### Requirement: El proceso final no corre con privilegios

El entrypoint MUST abandonar los privilegios y ejecutar `alloy` con el usuario
`app`; el proceso servidor MUST NOT correr como `root`.

#### Scenario: Proceso servidor sin privilegios

- **Given** un contenedor en ejecución
- **When** se inspecciona el usuario efectivo del proceso `alloy`
- **Then** el UID efectivo NO es 0 y coincide con `PUID` (o 1000 por defecto)

### Requirement: El directorio de datos es escribible por la identidad efectiva

El arranque MUST garantizar que el directorio `/app/data` (o el volumen montado
en él) sea escribible por el usuario `app` con la identidad configurada, incluso
si el volumen lo creó el host con otro propietario.

#### Scenario: Volumen preexistente con otro dueño

- **Given** un volumen `data/` en el host propiedad de UID/GID 1002 vacío
- **When** el contenedor arranca con `PUID=1002 PGID=1002`
- **Then** el proceso puede crear `alloy.db` y la app queda operativa
