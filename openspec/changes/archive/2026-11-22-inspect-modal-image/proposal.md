# Proposal

## Why

Issue #188 (follow-up de #166 parte 1). El change `container-row-ellipsis` dejó fuera la imagen del contenedor a propósito: `docs/container-row-ellipsis.md` documenta que la imagen no se pinta en la fila del listado y que su sitio es `InspectModal`.

El modal ya pinta `inspectData.image` y un bloque «Información adicional» con `containerInfo.image`, `image_tag` y un enlace «Ver en registry». Pero:

- El valor de la imagen puede ser largo (registry + ruta + tag) y **desborda** el layout del modal, sin elipsis ni tooltip.
- No hay **fallback**: si `inspectData.image` viene vacío, el campo queda en blanco aunque `containerInfo.image` + `image_tag` tengan el dato.
- El bloque «Información adicional» y el enlace a registry no tienen **tests** que fijen su comportamiento condicional.

## What Changes

- **Imagen principal con elipsis:** la fila «Imagen:» del bloque principal MUST truncar con elipsis real de Ant Design y mostrar el valor completo en tooltip. Debe respetar el ancho disponible (`minWidth: 0`) y no desbordar el modal.
- **Fallback de imagen:** cuando `inspectData.image` está vacío, se MUST mostrar `containerInfo.image` + `:` + `containerInfo.image_tag` en su lugar.
- **Bloque «Información adicional» condicional:** MUST renderizarse solo si `containerInfo` no es `null`. El enlace «Ver en registry» MUST renderizarse solo si `registry_url` no está vacío.
- **Tests:** escenarios colocados en `frontend/src/components/InspectModal.test.tsx` (elipsis, fallback, bloque condicional, enlace condicional).
- **Documentación:** `docs/inspect-modal-image.md` con el comportamiento y cómo verificarlo.

## Capabilities

### New Capabilities

- `ui/inspect-modal`: presentación de la imagen del contenedor en el modal de inspección (elipsis, tooltip, fallback y bloque de información adicional condicional).

### Modified Capabilities

Ninguna. `ui/container-row` queda intacta: la imagen sigue fuera de la fila del listado.

## Impact

- `frontend/src/components/InspectModal.tsx` (+ `frontend/src/components/InspectModal.test.tsx`).
- `docs/inspect-modal-image.md`.

### Fuera de alcance

- Mostrar la imagen en la fila del listado de contenedores.
- Cambios de backend o de contrato de API (`inspectData` y `containerInfo` se consumen tal cual).

Issue: #188.