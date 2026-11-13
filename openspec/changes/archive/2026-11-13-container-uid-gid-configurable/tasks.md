# Tasks

## 1. Verificación del estado previo (RED)

- [x] 1.1 Confirmar la identidad actual: `rg -n "adduser|USER|mkdir|chown" Dockerfile`
      muestra `adduser -D -h /app -u 1000 app`, sin GID explícito, `USER app`.
- [x] 1.2 Construir la imagen actual: `just build` (o `podman build -t localhost/alloy:red .`).
- [x] 1.3 Demostrar el fallo: `podman run --rm --entrypoint sh localhost/alloy:red -c 'id'`
      reporta `uid=1000(app) gid=1000(app)`; montar un `data/` con dueño `1002:1002`
      (`mkdirtmp`/`chown 1002:1002`) y ejecutar la app → error `CannotOpen` al abrir
      `data/alloy.db` (reproduce #83). Documentar salida.
- [x] 1.4 Confirmar que no existe entrypoint ni `su-exec`: `rg -n "ENTRYPOINT|su-exec" Dockerfile`
      no devuelve nada.

## 2. Dockerfile — identidad configurable en build

- [x] 2.1 Añadir `ARG UID=1000` y `ARG GID=1000` al stage de runtime (antes del `RUN apk add`).
- [x] 2.2 Crear grupo+usuario con GID explícito:
      `addgroup -g "$GID" app && adduser -D -h /app -u "$UID" -G app app`.
- [x] 2.3 Añadir `su-exec` al `apk add` del stage runtime.
- [x] 2.4 Propagar defaults: `ENV PUID=${UID} PGID=${GID}` (D3 del design).

## 3. Dockerfile — identidad configurable en runtime

- [x] 3.1 Crear `docker/entrypoint.sh`: leer `PUID`/`PGID` (default 1000), `chown -R
      "$PUID:$PGID" /app/data`, `exec su-exec "$PUID:$PGID" "$@"`; `set -e` y `chmod +x`.
- [x] 3.2 `COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh` (con `chmod +x`).
- [x] 3.3 Cambiar el runtime a `USER root` y añadir `ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]`,
      dejando `CMD ["./alloy"]` (el proceso final baja a `app` vía su-exec).
- [x] 3.4 Verificar que `.dockerignore` no excluya `docker/` (si aplica).

## 4. Verificación (GREEN)

- [x] 4.1 Build por defecto: `podman build -t localhost/alloy:green .`;
      `podman run --rm --entrypoint sh localhost/alloy:green -c 'id app'` → `uid=1000(app) gid=1000`.
- [x] 4.2 Build con identidad: `podman build --build-arg UID=1002 --build-arg GID=1002 -t localhost/alloy:1002 .`;
      `podman run --rm --entrypoint sh localhost/alloy:1002 -c 'id app'` → `uid=1002(app) gid=1002`.
- [x] 4.3 Drop de privilegios: `podman run --rm -e PUID=1002 -e PGID=1002 localhost/alloy:green id`
      → imprime `uid=1002` (el entrypoint ejecuta `su-exec 1002:1002 id`); NO `uid=0`.
- [x] 4.4 Defaults sin variables: `podman run --rm localhost/alloy:green id` → `uid=1000`.
- [x] 4.5 Escritura del volumen: `podman run --rm -v "$(mktemp -d):/app/data:Z" -e PUID=1002 -e PGID=1002
      localhost/alloy:green sh -c 'touch /app/data/probe && ls -ln /app/data/probe'` → fichero `1002 1002`.
- [x] 4.6 E2E real: levantar el contenedor con OIDC válido y volumen `1002:1002`; confirmar
      que crea `alloy.db` y sirve la UI sin error de SQLite.

## 5. Documentación y cierre

- [x] 5.1 Documentar en `docs/container-identity.md` (qué es, ficheros, cómo funciona,
      variables, cómo habilitar en build y en runtime, decisiones y follow-ups).
- [x] 5.2 Añadir `Environment=PUID/PGID` comentado a `alloy.container.example` con nota sobre `UserNS=keep-id`.
- [x] 5.3 Sección de despliegue en `README.md` (ajuste de UID/GID).
- [x] 5.4 `cd backend && just check` verde (no debe verse afectado, pero es pre-commit obligatorio).
- [x] 5.5 Archivar: `openspec archive container-uid-gid-configurable --yes` y verificar que
      `openspec/changes/container-uid-gid-configurable/` desaparece; si no, `rm -rf`.
