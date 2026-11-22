# dev/deploy-co1 Specification

## Purpose
Esta capability documenta el procedimiento de despliegue en co1: la imagen se fija a una etiqueta de versión explícita (nunca `:latest`), la verificación se hace por tag y el proceso es reproducible y auditable.

## Requirements

### Requirement: La imagen desplegada en co1 se pincha a una versión explícita

El stack `alloy` de co1 MUST referenciar la imagen `ghcr.io/atareao/alloy` con una **etiqueta de versión explícita** (`:X.Y.Z`, p. ej. `:0.26.0`). MUST NOT desplegar `:latest`. La documentación del deploy MUST reflejar que el workflow de release publica una etiqueta por versión además de `:latest`.

#### Scenario: El stack está pinchado a una versión

- **Given** el `compose.yaml` del stack `alloy` en co1
- **When** se revisa la referencia de la imagen
- **Then** la imagen usa `:X.Y.Z` (p. ej. `:0.26.0`)
- **And** no usa `:latest`

#### Scenario: La doc no contiene la afirmación falsa

- **Given** `docs/deploy-co1.md`
- **When** se busca la descripción de las etiquetas publicadas por el release
- **Then** la doc NO afirma que las releases «no publican tag por versión»
- **And** indica que se publican `:X.Y.Z` y `:latest`

### Requirement: La verificación del deploy se hace por tag

El procedimiento de verificación MUST permitir determinar qué versión corre comparando la **etiqueta** de la imagen en ejecución con la versión pinchada, en lugar de comparar digests o fechas.

#### Scenario: Verificación por tag

- **Given** un deploy reciente del stack `alloy`
- **When** se ejecuta la verificación documentada
- **Then** el tag reportado de la imagen coincide con la versión pinchada en el `compose.yaml`

### Requirement: El procedimiento de deploy es reproducible y auditable

El repositorio MUST contener un **ejemplo versionado** del compose del stack de co1 con la imagen pinchada a una versión, de modo que el procedimiento pueda reproducirse sin depender de un estado no versionado del host.

#### Scenario: Ejemplo de compose versionado

- **Given** el repositorio clonado
- **When** se busca el ejemplo del stack de co1
- **Then** existe `deploy/co1.compose.example.yaml` con la imagen pinchada a `:X.Y.Z`
