# Design

## Context

- Imagen multi-stage Alpine (`Dockerfile`), runtime `alpine:3.23`, binario estático
  en `/app/alloy`, datos en `/app/data` (volumen), `CMD ["./alloy"]`, hoy `USER app`.
- El repositorio **no** publica `docker-compose.yml`; el despliegue de referencia es un
  Quadlet de Podman (`alloy.container.example`) con `UserNS=keep-id`.
- La imagen publicada (`ghcr.io/atareao/alloy`) la construye `release.yml` sin `build-args`.
- El reporter del issue #83 usa la imagen publicada con el socket de Podman y un volumen
  con GID 1002; no puede reconstruir para su host de forma cómoda.

## Goals / Non-Goals

**Goals:**

- Permitir que la identidad del proceso coincida con la del host, tanto al construir la
  imagen propia como al ejecutar la imagen publicada.
- Conservar 1000/1000 como default (sin regresión para quien ya funciona).
- Garantizar que `/app/data` sea escribible por la identidad efectiva.
- No dejar el proceso servidor corriendo como `root`.

**Non-Goals:**

- No se añade un `docker-compose.yml` al repo.
- No se toca la lógica de la app (Rust) ni la ruta `data/alloy.db`.
- No se gestionan remapeos de usuario de Podman más allá de documentar `UserNS=keep-id`.

## Decisions

### D1 — Doble vía: build-time (ARG) y runtime (PUID/PGID)

Se soportan **ambas** porque resuelven a públicos distintos:

- `ARG UID`/`ARG GID` (default 1000) permiten a quien construye la imagen con
  `just build` fijar la identidad del host en el momento del build.
- `PUID`/`PGID` en runtime resuelven el caso del issue: usuario de la imagen
  *publicada* que no puede reconstruir.

*Alternativa considerada*: solo build-ARG. Rechazada: obliga a reconstruir la imagen
publicada para ajustar la identidad, que es justo lo que el issue pide evitar.

### D2 — Entrypoint que baja privilegios con `su-exec`

El runtime stage instala `su-exec` (paquete Alpine, diminuto), arranca como `root` y un
`ENTRYPOINT` hace, antes de `exec`:

1. Resolver `PUID`/`PGID` (default = los del build, horneados vía `ENV`).
2. `chown -R "$PUID:$PGID" /app/data` (el volumen puede venir con otro dueño).
3. `exec su-exec "$PUID:$PGID" ./alloy`.

`su-exec` acepta UID:GID **numéricos**, por lo que **no** se necesita `usermod`/`groupmod`
ni el paquete `shadow`: no se modifica la base de usuarios, solo se cambia de identidad al
lanzar el proceso.

*Alternativa considerada*: `USER app` sin entrypoint. Rechazada: no permite ajustar la
identidad sin reconstruir (D1). *Alternativa considerada*: `usermod`/`groupmod` en un
entrypoint root. Rechazada por complejidad innecesaria (dependencia `shadow`, reescritura
de `/etc/passwd`).

### D3 — Defaults horneados con `ENV` a partir de los `ARG`

`ARG UID=1000` / `ARG GID=1000` se propagan con `ENV PUID=$UID PGID=$GID` en el stage de
runtime, de modo que una imagen construida con `--build-arg UID=1002` arranca con
`PUID=1002` aunque no se pase ninguna variable. Quien quiera lo sobreescribe en runtime.

### D4 — El `alloy.container.example` documenta, no impone

Se añade `Environment=PUID=...`/`PGID=...` comentado al Quadlet de ejemplo con una nota
sobre la interacción con `UserNS=keep-id`, sin cambiar el default del ejemplo.

## Risks / Trade-offs

- **Arranque como root + drop**: el contenedor inicia como `root` y baja a `app` antes de
  `exec`. Es el patrón estándar (linuxserver.io) pero cambia la postura de seguridad
  respecto a `USER app`. Mitigación: el entrypoint es mínimo y no ejecuta nada más;
  el proceso servidor nunca corre como root (Requirement 3).
- **`chown -R` en cada arranque**: coste proporcional al tamaño de `/app/data`
  (JSON + SQLite, pequeño). Aceptable; alternativa (chown condicional) se evita por
  simplicidad y correctitud.
- **`UserNS=keep-id` en Podman**: con keep-id el mapeo ya alinea UID del host y del
  contenedor; `PUID`/`PGID` y keep-id pueden solaparse. Se documenta, no se automatiza.
- **Superficie del entrypoint**: nueva dependencia `su-exec` en la imagen y un script
  shell. Riesgo bajo; se verifica en TDD (GREEN) ejecutando el contenedor con distintas
  identidades.
