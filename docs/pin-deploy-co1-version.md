# pin-deploy-co1-version — Deploy de co1 reproducible

- **Issue**: #156
- **Change OpenSpec**: `pin-deploy-co1-version` (capability `dev/deploy-co1`)
- **Plan**: `plans/PLAN-012.md`

## Qué es

Corrección del procedimiento de despliegue de Alloy en co1 para que sea **reproducible**: la imagen se pincha a un **tag de versión explícito** (`:X.Y.Z`) en vez de `:latest`, y la verificación se hace **por tag**.

## Por qué

`docs/deploy-co1.md` afirmaba erróneamente que las releases publican solo `:latest`. Es **falso**: `.github/workflows/release.yml` publica `:${VERSION}` **y** `:latest`. Desplegar `:latest` hacía imposible saber qué versión corre.

## Ficheros implicados

- `docs/deploy-co1.md` — procedimiento corregido (pin + verificación por tag).
- `deploy/co1.compose.example.yaml` — ejemplo versionado del stack con la imagen pinchada.
- `openspec/specs/dev/deploy-co1/spec.md` — contrato (capability).

## Cómo funciona

El stack de co1 (Dockge) referencia `ghcr.io/atareao/alloy:X.Y.Z`. Al publicar una release, GHCR recibe `:X.Y.Z` y `:latest`; el operador pincha el tag nuevo en `compose.yaml`, hace `docker compose up -d` y verifica con `docker inspect alloy --format '{{.Config.Image}}'`.

## Variables / configuración

- El tag de versión en `compose.yaml` (`image: ghcr.io/atareao/alloy:X.Y.Z`).
- `.env` del stack (`FQDN`, `DOCKER_HOST`, `OIDC_*`).

## Cómo habilitar / desplegar

Ops manual en co1 (ver `docs/deploy-co1.md`):

```sh
ssh co1 && cd /home/lorenzo/docker/dockge/stacks/alloy
# editar compose.yaml → image: ghcr.io/atareao/alloy:0.28.0
docker pull ghcr.io/atareao/alloy:0.28.0 && docker compose up -d
docker inspect alloy --format '{{.Config.Image}}'
```

## Decisiones

- **Pin por tag, no por digest**: el tag es legible y auditable; el digest cambia con cada rebuild del mismo tag.
- **Ejemplo versionado en el repo**: hace el procedimiento reproducible sin depender del estado del host.

## Follow-ups

- **#160**: README desactualizado (React 18/Mantine/JSON) — el quickstart aún usa `:latest`.
- El tag `:latest` sigue publicándose para conveniencia, pero **no** es la referencia de deploy.
