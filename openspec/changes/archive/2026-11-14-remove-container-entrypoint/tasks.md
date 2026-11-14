# Tasks

## 1. RED — Evidencia del bug y del comportamiento correcto

- [x] 1.1 Evidencia del incidente: el proceso `alloy` con `group_add` carece del GID del socket
      (`Groups: 1000`) y no puede conectar con Docker. Guardar salida.
- [x] 1.2 Evidencia del comportamiento con `USER app` (sin entrypoint): el proceso conserva los
      grupos suplementarios y abre el socket.

## 2. GREEN — Retirar el entrypoint

- [x] 2.1 `Dockerfile`: stage de runtime → `USER app`; retirar `su-exec`/entrypoint/`ENV PUID/PGID`;
      conservar `ARG UID/GID` y el `chown` de `/app/data`.
- [x] 2.2 Borrar `docker/entrypoint.sh`.
- [x] 2.3 `docs/container-identity.md`, `README.md`, `alloy.container.example`: retirar `PUID`/`PGID`.

## 3. REFACTOR y verificación

- [x] 3.1 `rg -n 'su-exec|entrypoint|PUID|PGID' Dockerfile docs/ README.md alloy.container.example`
      sin referencias obsoletas.
- [x] 3.2 Test real (docker): construir/reutilizar y comprobar que sin entrypoint el proceso es `app`
      y que con `--group-add <gid-socket>` abre `/var/run/docker.sock`.

## 4. Documentación y cierre

- [x] 4.1 Auditar, abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
