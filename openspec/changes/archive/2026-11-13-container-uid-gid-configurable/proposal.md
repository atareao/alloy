# Proposal

## Why

La imagen fija la identidad del proceso: `adduser -D -h /app -u 1000 app`
(`Dockerfile:53`) y arranca con `USER app` (UID/GID 1000). En un host cuyo usuario
no es 1000 (el issue #83 reporta GID 1002) el volumen `data/` montado no es
escribible por el proceso y SQLite falla al abrir la base con `CannotOpen`
(`main.rs:65`). Hoy no hay forma de ajustar esa identidad sin parchear y
reconstruir la imagen a mano.

## What Changes

- El par **UID/GID** de la imagen pasa a ser configurable, conservando **1000 por
  defecto** (compatibilidad total con quien ya la usa).
- **En build**: el stage de runtime acepta `ARG UID`/`ARG GID` para construir una
  imagen con la identidad del host (`podman build --build-arg UID=$(id -u) ...`).
- **En runtime**: el contenedor lee las variables `PUID`/`PGID`; un `entrypoint`
  reajusta el usuario `app`, cede la propiedad de `/app/data` y **baja
  privilegios** antes de `exec ./alloy`. Sin variables, comportamiento idéntico
  al actual.
- Se mantiene `mkdir -p /app/data && chown` del build y se añade la reasignación
  de propiedad en el arranque (el volumen puede venir con otro dueño).
- Se documenta la nueva variable en el Quadlet de ejemplo (`alloy.container.example`)
  y en el `README.md`.

## Capabilities

### New Capabilities

- `build/container-identity`: la imagen arranca con una identidad de usuario (UID/GID)
  configurable en build y en runtime, garantizando que `/app/data` sea escribible
  por esa identidad.

### Modified Capabilities

Ninguna. El comportamiento del producto no cambia; `build/docker-context` describe
el contexto de build (node_modules, pnpm no interactivo), no la identidad del proceso.

## Impact

**Ficheros afectados**

- `Dockerfile` — stage runtime: `ARG UID/GID`, `groupadd/adduser` con GID explícito,
  `COPY` del entrypoint, `USER root` + `ENTRYPOINT`.
- `docker/entrypoint.sh` (nuevo) — reajuste de UID/GID y `su-exec`.
- `alloy.container.example` — documenta `Environment=PUID/PGID`.
- `README.md` — sección de despliegue: cómo ajustar UID/GID.
- `docs/` (nuevo `docs/container-identity.md`) — documentación de la feature.

**Sin cambios en**: `backend/src/**`, `frontend/src/**`.

**Coste / CI**: `release.yml` sigue construyendo con los defaults (build sin
`build-args` obligatorios); no añade pasos ni tiempo de CI relevante.
