# build/container-identity Specification

## Purpose
La imagen Docker de Alloy debe poder arrancar con una identidad de usuario
(UID/GID) ajustable al host, sin dejar de correr sin privilegios ni perder la
compatibilidad con el despliegue por defecto.

## Requirements

### Requirement: La identidad del proceso es configurable en build

El stage de runtime MUST aceptar los `ARG UID` y `ARG GID` (por defecto `1000`) y crear el usuario
`app` con esa identidad, de modo que la imagen pueda construirse con el UID/GID de cualquier host.
Esta es la **única** vía de configuración de identidad: no MUST existir reajuste en runtime.

#### Scenario: Build con la identidad del host

- **Given** un host cuya cuenta es UID 1002 / GID 1002
- **When** se construye la imagen con `--build-arg UID=1002 --build-arg GID=1002`
- **Then** dentro de la imagen `id app` reporta `uid=1002(app) gid=1002`

#### Scenario: Build por defecto

- **Given** un build sin `--build-arg`
- **When** se construye la imagen
- **Then** el usuario `app` queda con UID 1000 y GID 1000

#### Scenario: Sin reajuste en runtime

- **Given** un contenedor arrancado con `PUID`/`PGID` definidos en el entorno
- **When** arranca
- **Then** esos valores se ignoran y el proceso corre con la identidad del build

### Requirement: El proceso final no corre con privilegios

El contenedor MUST ejecutar `alloy` directamente como el usuario `app` (`USER app`), sin un
entrypoint que arranque como `root`; el proceso servidor MUST NOT correr como `root`.

#### Scenario: Proceso servidor sin privilegios

- **Given** un contenedor en ejecución
- **When** se inspecciona el usuario efectivo del proceso `alloy`
- **Then** el UID efectivo NO es 0

### Requirement: Los grupos suplementarios del contenedor llegan al proceso

Al no haber entrypoint, el proceso `alloy` MUST conservar los grupos suplementarios que el
contenedor reciba en runtime (`--group-add`, `group_add`, `GroupAdd=`), de modo que pueda acceder a
recursos del host como `/var/run/docker.sock` (habitualmente `root:docker`).

#### Scenario: Acceso al socket con group_add

- **Given** un contenedor arrancado con el socket de Docker montado y su GID concedido con
  `--group-add <gid-socket>`
- **When** arranca la app
- **Then** el proceso `alloy` tiene `<gid-socket>` entre sus grupos suplementarios
- **And** puede abrir `/var/run/docker.sock` y listar contenedores

### Requirement: El volumen de datos nuevo es escribible por app

La imagen MUST crear `/app/data` con la propiedad del usuario `app`, de modo que un volumen de datos
**nuevo** (que hereda la propiedad del directorio de la imagen al montarse) sea escribible por la
app.

#### Scenario: Volumen nuevo escribible

- **Given** un contenedor arrancado con un volumen de datos vacío montado en `/app/data`
- **When** la app arranca
- **Then** puede crear `alloy.db` en `/app/data` sin error
