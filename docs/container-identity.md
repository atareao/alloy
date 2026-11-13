# container-identity

## Qué es

La imagen de Alloy puede arrancar con una identidad de usuario (UID/GID) distinta de
la de por defecto (1000/1000), para que el proceso tenga permiso de escritura sobre el
volumen `data/` del host. Resuelve el issue
[#83](https://github.com/atareao/alloy/issues/83): con la identidad fija, un host cuyo
usuario no es 1000 provocaba `CannotOpen` al abrir `data/alloy.db`.

## Ficheros implicados

- `Dockerfile` — stage de runtime: `ARG UID/GID`, creación de `app` con GID explícito,
  `su-exec`, `ENV PUID/PGID`, `USER root` + `ENTRYPOINT`.
- `docker/entrypoint.sh` — cede la propiedad de `/app/data` y baja privilegios con
  `su-exec` antes de lanzar la app.
- `alloy.container.example` — variables `PUID`/`PGID` documentadas.
- `README.md` — despliegue y tabla de variables.

## Cómo funciona

1. En **build**, `ARG UID` y `ARG GID` (default 1000) crean el usuario `app` con esa
   identidad. Además se hornean como default en `ENV PUID`/`ENV PGID`.
2. Al **arrancar**, el contenedor empieza como `root` y `docker/entrypoint.sh`:
   - resuelve `PUID`/`PGID` (default = los del build),
   - ejecuta `chown -R "$PUID:$PGID" /app/data` (el volumen puede venir con otro dueño),
   - hace `exec su-exec "$PUID:$PGID" "$@"`, de modo que el proceso servidor **nunca**
     corre como `root`.
3. `su-exec` recibe UID:GID numéricos: no se modifica `/etc/passwd` ni se necesita
   `shadow`/`usermod`.

## Variables / configuración

| Variable | Default | Descripción |
|---|---|---|
| `PUID` | `1000` (o el `ARG UID` del build) | UID con el que corre `alloy` |
| `PGID` | `1000` (o el `ARG GID` del build) | GID con el que corre `alloy` |

## Cómo habilitar / desplegar

**Imagen publicada** (sin reconstruir):

```sh
podman run -d --name alloy -p 3066:3066 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v alloy-data:/app/data \
  -e PUID=$(id -u) -e PGID=$(id -g) \
  ... ghcr.io/atareao/alloy:latest
```

**Imagen propia** (identidad hornada en el build):

```sh
podman build --build-arg UID=$(id -u) --build-arg GID=$(id -g) -t localhost/alloy:latest .
```

**Quadlet**: añade `Environment=PUID=...` / `Environment=PGID=...`. Con
`UserNS=keep-id` el mapeo ya alinea la identidad del host; ambas vías conviven, pero
no es necesario usar las dos a la vez.

## Decisiones

- Doble vía build-ARG + runtime `PUID`/`PGID` (design D1): la primera sirve a quien
  construye su imagen; la segunda al usuario de la imagen publicada.
- `su-exec` con IDs numéricos en vez de `usermod`/`groupmod` (design D2).
- Defaults horneados con `ENV` desde los `ARG` (design D3), para que una imagen
  construida con `--build-arg UID=1002` arranque como 1002 sin variables.

## Follow-ups

- Valorar `chown` condicional en el entrypoint si `/app/data` crece mucho (hoy es
  pequeño: JSON + SQLite).
- Revisar la interacción documentada entre `PUID/PGID` y `UserNS=keep-id` si se
  adopta keep-id por defecto en el ejemplo.
