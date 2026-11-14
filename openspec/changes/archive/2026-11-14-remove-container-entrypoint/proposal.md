# Proposal

## Why

La imagen actual arranca el proceso servidor con un entrypoint (`docker/entrypoint.sh`) que baja
privilegios con `su-exec "$PUID:$PGID"`. **`su-exec` fija los grupos suplementarios del proceso al
GID indicado**, descartando el resto — incluido el grupo del socket de Docker que el contenedor
recibe en runtime (`--group-add`/`group_add`/`GroupAdd=`). Resultado: la app no puede conectar con
Docker (*"Docker events stream error: … client error (Connect)"*) y el dashboard queda **vacío**.

Es una **regresión**: ese entrypoint se introdujo ayer (commit `3b13600`, issue #83) y la imagen
anterior —`USER app` directo, sin entrypoint— aplicaba `group_add` correctamente.

La capacidad que justificaba el entrypoint (`PUID`/`PGID` en **runtime**) no aporta valor que
compense su coste y su superficie de error: la identidad ya es configurable en **build**
(`--build-arg UID/GID`) y los despliegues con usuario no-1000 disponen además de `UserNS=keep-id`.
Se retira el entrypoint y se vuelve a `USER app`.

## What Changes

- `Dockerfile`: el stage de runtime vuelve a `USER app` (sin `su-exec`, sin `setpriv`, sin
  entrypoint, sin `ENV PUID/PGID`). Se conserva la configurabilidad **en build** (`ARG UID/GID`).
- Se elimina `docker/entrypoint.sh`.
- `docs/container-identity.md`: reescrito (identidad en build; `group_add` da acceso al socket).
- `README.md` y `alloy.container.example`: retiran `PUID`/`PGID`; documentan `--build-arg` y
  `GroupAdd`.

## Capabilities

### Modified Capabilities

- `build/container-identity`:
  - **REMOVED**: el requisito de identidad configurable en **runtime** (`PUID`/`PGID`) y el de
    "el directorio de datos es escribible por la identidad efectiva" (su garantía dependía del
    reajuste en runtime del entrypoint).
  - **MODIFIED**: el de identidad configurable en **build** (única vía) y el de "el proceso final no
    corre con privilegios" (`USER app`, sin entrypoint).
  - **ADDED**: "los grupos suplementarios del contenedor llegan al proceso" (`group_add` →
    `/var/run/docker.sock`) y "el volumen de datos nuevo es escribible por app".

## Impact

- `Dockerfile`, `docker/entrypoint.sh` (borrado), `docs/container-identity.md`, `README.md`,
  `alloy.container.example`.
- **Reversión de parte de #83**: los hosts cuyo usuario no sea 1000 deberán construir con
  `--build-arg UID=$(id -u) --build-arg GID=$(id -g)` o usar `UserNS=keep-id`. En el host de
  producción (usuario 1000) no cambia nada y se puede retirar el workaround `PGID=999`.
- Requiere un **release patch** de la imagen.
