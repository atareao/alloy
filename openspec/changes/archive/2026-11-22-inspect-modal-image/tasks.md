# Tasks

## TDD Checklist

### RED

- [x] Añadir test de elipsis/tooltip de `inspectData.image` en `frontend/src/components/InspectModal.test.tsx` (texto completo en DOM + clase `ant-typography-ellipsis`).
- [x] Añadir test de fallback `containerInfo.image:image_tag` cuando `inspectData.image` está vacío.
- [x] Añadir test de bloque «Información adicional» ausente con `containerInfo` a `null`.
- [x] Añadir test del enlace «Ver en registry» ausente con `registry_url` vacío y presente cuando hay valor.
- [x] Ejecutar `npx vitest run` y confirmar los fallos esperados.

### GREEN

- [x] Aplicar `ellipsis` + tooltip y `minWidth: 0` a la imagen principal de `InspectModal.tsx`.
- [x] Implementar el fallback a `image:tag` cuando `inspectData.image` está vacío.
- [x] Asegurar el renderizado condicional del bloque y del enlace a registry.
- [x] Ejecutar `npx vitest run` y confirmar que pasan los tests nuevos y los existentes.

### REFACTOR

- [x] Extraer un helper de etiqueta/valor reutilizado por las filas del modal, sin cambiar comportamiento.
- [x] Revisar estilos (ancho de etiqueta, `minWidth: 0`, `flexShrink`) para mantener consistencia con `ContainerRow`.

### Verificación

- [x] `pnpm exec tsc -b` → 0.
- [x] `pnpm run lint` → 0.
- [x] `npx vitest run` → 0.
- [x] Documentar en `docs/inspect-modal-image.md`.
- [ ] Cerrar el issue #188 con referencia al change.