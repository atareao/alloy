# ui/inspect-modal Specification

## Purpose
TBD - created by archiving change inspect-modal-image. Update Purpose after archive.

## Requirements

### Requirement: La imagen del contenedor SHALL mostrarse truncada con tooltip

La fila «Imagen:» del bloque principal de `InspectModal` MUST renderizar el valor de `inspectData.image` con elipsis real de Ant Design (`Typography.Text` con `ellipsis`). Cuando el texto no cabe en el ancho disponible, MUST mostrarse truncado con `…` y el valor completo MUST estar disponible en un tooltip. El texto MUST NOT desbordar el contenedor del modal.

El texto MUST poder encoger dentro de su `Flex` (por ejemplo con `minWidth: 0`), y el resto de la fila MUST NOT impedir la elipsis.

#### Scenario: Imagen larga se trunca y expone el valor completo

- **Given** un `InspectData` con una imagen de registry larga
- **When** se abre `InspectModal` con esos datos
- **Then** la imagen se muestra con la clase de elipsis (`ant-typography-ellipsis`)
- **And** el texto completo de la imagen sigue presente en el DOM
- **And** el valor completo se ofrece como tooltip cuando el texto no cabe

#### Scenario: La imagen no desborda el contenedor

- **Given** un `InspectModal` con una imagen muy larga y un viewport estrecho
- **When** se renderiza el modal
- **Then** el texto de la imagen no excede el ancho de la fila
- **And** no aparece scroll horizontal por la imagen

### Requirement: La imagen SHALL tener fallback cuando `inspectData.image` falta

Si `inspectData.image` es una cadena vacía o no está disponible, el campo «Imagen:» MUST mostrar el valor compuesto `containerInfo.image` + `:` + `containerInfo.image_tag`. Si tampoco hay `containerInfo`, el campo MUST mostrar un marcador vacío sin romper el resto del modal.

#### Scenario: Fallback a `image:tag`

- **Given** un `InspectData` sin imagen (`image` vacío)
- **And** un `containerInfo` con `image` `nginx` y `image_tag` `latest`
- **When** se abre `InspectModal`
- **Then** la fila «Imagen:» muestra `nginx:latest`

#### Scenario: Sin imagen ni `containerInfo`

- **Given** un `InspectData` sin imagen
- **And** `containerInfo` es `null`
- **When** se abre `InspectModal`
- **Then** el modal se renderiza sin errores
- **And** la fila «Imagen:» queda vacía

### Requirement: El bloque de información adicional SHALL ser condicional

El bloque «Información adicional» MUST renderizarse solo cuando `containerInfo` no sea `null`. Con `containerInfo` en `null`, el bloque y sus filas (imagen, tag, registry) MUST NOT aparecer.

El enlace «Ver en registry» MUST renderizarse solo cuando `containerInfo.registry_url` no esté vacío. Con `registry_url` vacío, la fila «Registry:» MUST NOT aparecer.

#### Scenario: Sin `containerInfo` no hay bloque

- **Given** un `InspectData` válido y `containerInfo` a `null`
- **When** se abre `InspectModal`
- **Then** no aparece el texto «Información adicional»
- **And** no aparece la fila «Registry:»

#### Scenario: Sin `registry_url` no hay enlace

- **Given** un `containerInfo` con `image` y `image_tag` pero `registry_url` vacío
- **When** se abre `InspectModal`
- **Then** sí aparece el bloque «Información adicional»
- **And** no aparece el enlace «Ver en registry»

#### Scenario: Con `registry_url` aparece el enlace

- **Given** un `containerInfo` con `registry_url` no vacío
- **When** se abre `InspectModal`
- **Then** aparece el enlace «Ver en registry»
- **And** su `href` es `registry_url`

### Requirement: Los escenarios SHALL cubrirse con tests

Los escenarios anteriores MUST cubrirse en `frontend/src/components/InspectModal.test.tsx` con `@testing-library/react`. La verificación MUST ejecutar `npx vitest run` en verde.

#### Scenario: La suite cubre elipsis, fallback y bloques condicionales

- **Given** los tests de `InspectModal.test.tsx`
- **When** se ejecuta `npx vitest run`
- **Then** existe al menos un test por cada uno de los escenarios anteriores
- **And** la suite termina con código 0
