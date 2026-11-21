# Deploy en co1 (Dockge)

Procedimiento para (re)desplegar Alloy en el servidor `co1` **pinchando una versión concreta**.

## Entorno

- **Host**: `co1` (Contabo), accesible por SSH (`ssh co1`).
- **Runtime**: Docker 28.2.2, stack gestionado por **Dockge** (`louislam/dockge`).
- **Stack**: `alloy` → `/home/lorenzo/docker/dockge/stacks/alloy/compose.yaml` (+ `.env`).
- **Imagen**: `ghcr.io/atareao/alloy`, **pinchada a una versión** (p. ej. `:0.26.0`).
- **Dominio**: `https://alloy.territoriolinux.es` (Traefik, red `proxy`).
- **Socket**: `/var/run/docker.sock` montado; `group_add: ["999"]` (GID del socket) para el usuario no privilegiado `app`.

## Requisitos

- La versión objetivo debe estar **publicada** en GHCR. El workflow de release publica **dos** etiquetas por release: `ghcr.io/atareao/alloy:X.Y.Z` **y** `ghcr.io/atareao/alloy:latest` (`.github/workflows/release.yml`).
- El stack **debe** referenciar la imagen por su **tag de versión** (`:X.Y.Z`). **NUNCA** `:latest`: haría el deploy no reproducible (no se sabría qué versión corre).

## Pinchar la versión

1. Elegir la versión objetivo `X.Y.Z` (p. ej. `0.26.0`), la última release publicada.
2. Editar `compose.yaml` y fijar la imagen:

```yaml
services:
  alloy:
    image: ghcr.io/atareao/alloy:0.26.0
```

## Procedimiento

```sh
ssh co1
cd /home/lorenzo/docker/dockge/stacks/alloy

# 1. Traer la imagen de la versión pinchada
docker pull ghcr.io/atareao/alloy:0.26.0

# 2. Redeploy (recrea el contenedor con la nueva imagen)
docker compose up -d

# 3. Verificar (por TAG, no por digest)
docker ps --format '{{.Names}} {{.Image}} {{.Status}}' | grep -i alloy
docker inspect alloy --format '{{.Config.Image}}'   # debe imprimir ghcr.io/atareao/alloy:0.26.0
curl -s -o /dev/null -w '%{http_code}\n' https://alloy.territoriolinux.es/
```

> `docker inspect alloy --format '{{.Config.Image}}'` devuelve el **tag** con el que se creó el contenedor. Compara ese tag con la versión pinchada en `compose.yaml`: deben coincidir. No compares digests ni fechas.

## Verificación de salud

Logs esperados al arrancar: `🔐 OIDC`, `✅ OIDC discovery`, `JWKS fetched`, `🔌 Conectando a Docker`, `🚀 Alloy en http://0.0.0.0:3066`. HTTP externo `200`.

## Actualizar a una versión nueva

1. Editar `compose.yaml` a la nueva versión (`:X.Y.Z`).
2. `docker pull ghcr.io/atareao/alloy:X.Y.Z && docker compose up -d`.
3. Verificar por tag (arriba).

## Limpieza (opcional)

```sh
docker image prune -f   # elimina imágenes antiguas huérfanas
```

## Notas

- `compose.yaml` usa variables de `.env` (`FQDN`, `DOCKER_HOST`, `OIDC_*`). No borrar `.env`.
- Ejemplo versionado del stack: [`deploy/co1.compose.example.yaml`](../deploy/co1.compose.example.yaml).
- Hay backups `compose.yaml.bak-*` en el directorio del stack.
- Desplegar la **última release** = pinchar el tag de la última release publicada (no `:latest`).
