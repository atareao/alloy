# container-identity

## Qué es

La identidad del proceso `alloy` se fija en **build**. El stage de runtime del `Dockerfile`
acepta `ARG UID`/`ARG GID` (por defecto 1000) y crea el usuario `app` con esa identidad, de
modo que un host cuyo usuario no sea 1000 pueda escribir el volumen `data/`.

No hay reajuste de identidad en **runtime**: el contenedor no arranca como `root` ni usa un
entrypoint. Es deliberado (issue [#120](https://github.com/atareao/alloy/issues/120)): un
entrypoint que bajaba privilegios con `su-exec` **descartaba los grupos suplementarios**
concedidos con `group_add`/`--group-add`/`GroupAdd=`, rompiendo el acceso a recursos del host
como `/var/run/docker.sock`.

## Ficheros implicados

- `Dockerfile` — stage de runtime: `ARG UID/GID`, creación de `app` con GID explícito y `USER app`.
- `README.md` — despliegue y tabla de variables.
- `alloy.container.example` — `GroupAdd` para el grupo del socket.

## Cómo funciona

1. En **build**, `ARG UID` y `ARG GID` (default 1000) crean el usuario `app` con esa identidad.
2. El proceso servidor arranca directamente como `app` (`USER app`), **sin entrypoint** y **sin** correr como `root`.
3. Los grupos suplementarios que el contenedor reciba en runtime (`--group-add`, `group_add` o `GroupAdd=` del Quadlet) llegan intactos al proceso, así que puedes conceder acceso al socket de Docker (`root:docker`, GID 999/136 típicamente) sin correr el contenedor como `root`.

## Cómo habilitar / desplegar

**Acceso al socket de Docker** (para que Alloy vea los contenedores): concede al contenedor el GID del socket del host.

El GID (`999`, `136`, …) es el del grupo dueño de `/var/run/docker.sock` en tu host
(`999` suele ser `docker`); usa el que corresponda a tu sistema.

```yaml
# docker compose
group_add: ["999"]
```

```sh
# docker run
--group-add 999
```

```ini
# Quadlet
GroupAdd=136
```

**Host con usuario distinto de 1000**: construye tu propia imagen con tu identidad:

```sh
podman build --build-arg UID=$(id -u) --build-arg GID=$(id -g) -t localhost/alloy:latest .
```

o usa `UserNS=keep-id` en Podman/Quadlet, que alinea la identidad del host.

## Decisiones

- Identidad solo en **build** (`ARG UID/GID`); sin reajuste en runtime. Un entrypoint que baja privilegios con `su-exec` anula `group_add` (issue #120), así que se retiró.
- El proceso corre con `USER app` directamente: menos modos de fallo y compatibilidad con `group_add`/`GroupAdd`.

## Follow-ups

- Un volumen `data/` preexistente creado por otro UID/GID requiere construirse con la identidad acorde o `UserNS=keep-id` (ya no se auto-corrige en runtime).
