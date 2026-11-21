# Deploy en co1 (Dockge)

Procedimiento para (re)desplegar Alloy en el servidor `co1` a la **última release**.

## Entorno

- **Host**: `co1` (Contabo), accesible por SSH (`ssh co1`).
- **Runtime**: Docker 28.2.2, stack gestionado por **Dockge** (`louislam/dockge`).
- **Stack**: `alloy` → `/home/lorenzo/docker/dockge/stacks/alloy/compose.yaml` (+ `.env`).
- **Imagen**: `ghcr.io/atareao/alloy` (tag `:latest`, publicada por el workflow de release).
- **Dominio**: `https://alloy.territoriolinux.es` (Traefik, red `proxy`).
- **Socket**: `/var/run/docker.sock` montado; `group_add: ["999"]` (GID del socket) para el usuario no privilegiado `app`.

## Requisitos

- La release debe estar **publicada** (merge a `main` → `release-prepare.yml` construye y publica `:latest`).
- Las releases publican `:latest` (no tag por versión), así que se compara por **fecha/digest** de la imagen.

## Procedimiento

```sh
ssh co1
cd /home/lorenzo/docker/dockge/stacks/alloy

# 1. Traer la última imagen
docker pull ghcr.io/atareao/alloy:latest

# 2. Redeploy (recrea el contenedor si el digest cambió)
docker compose up -d

# 3. Verificar
docker ps --format '{{.Names}} {{.Image}} {{.Status}}' | grep -i alloy
docker inspect alloy --format 'Image={{.Image}} Created={{.Created}}'
docker logs alloy 2>&1 | tail
curl -s -o /dev/null -w '%{http_code}\n' https://alloy.territoriolinux.es/
```

## Verificación de salud

Logs esperados al arrancar: `🔐 OIDC`, `✅ OIDC discovery`, `JWKS fetched`, `🔌 Conectando a Docker`, `🚀 Alloy en http://0.0.0.0:3066`. HTTP externo `200`.

## Limpieza (opcional)

```sh
docker image prune -f   # elimina la imagen antigua huérfana
```

## Notas

- `compose.yaml` usa variables de `.env` (`FQDN`, `DOCKER_HOST`, `OIDC_*`). No borrar `.env`.
- Hay backups `compose.yaml.bak-*` en el directorio del stack.
- Desplegar la **última release** = lo que esté en `:latest` (puede ir por detrás de `develop`).
