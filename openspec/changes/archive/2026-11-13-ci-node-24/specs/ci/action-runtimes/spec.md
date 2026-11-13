# Spec Delta — ci/action-runtimes

## ADDED Requirements

### Requirement: Ninguna action usa un runtime Node deprecado

Las actions basadas en Node usadas en `.github/workflows/` MUST declarar un runtime Node 24 o
superior. Ninguna MUST declarar Node 20 (deprecado) ni versiones anteriores. Las actions
*composite* (que no declaran runtime Node) quedan fuera de este requisito.

#### Scenario: Ejecución sin warnings de deprecación de Node

- **Given** los workflows de CI y release con las actions actualizadas
- **When** se ejecuta cualquiera de ellos
- **Then** no aparece el warning "Node.js 20 is deprecated"
- **And** ninguna action se ve forzada a un runtime distinto del que declara

#### Scenario: El pipeline de release sigue funcionando

- **Given** las actions actualizadas en `release.yml` y `release-prepare.yml`
- **When** se dispara un release (push de tag `v*` o `workflow_dispatch`)
- **Then** los jobs `build`, `release` y `docker` terminan con éxito
- **And** el tag se publica con sus binarios y la imagen en GHCR
