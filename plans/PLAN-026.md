# PLAN-026 — Imagen del contenedor en InspectModal

- Issue: #188 (follow-up de #166 parte 1)
- Tipo: feature frontend (incremental)
- Change OpenSpec: `openspec/changes/inspect-modal-image/`
- Doc final: `docs/inspect-modal-image.md`

## Objetivo

Endurecer el render de la imagen en `InspectModal` y cubrirlo con tests. No se
muestra la imagen en la fila del listado (fuera de alcance, ver
`docs/container-row-ellipsis.md`).

## Alcance

1. Imagen principal (`inspectData.image`) con truncado por ellipsis y tooltip con
   el valor completo; sin desbordar el layout en móvil.
2. Fallback: si `inspectData.image` está vacío o ausente, mostrar
   `containerInfo.image:containerInfo.image_tag`.
3. Bloque «Información adicional» (imagen, tag, registry) solo si `containerInfo`
   no es null; el enlace «Ver en registry» solo si `registry_url` no está vacío.
4. Tests colocados en `InspectModal.test.tsx`: imagen principal, fallback,
   bloque adicional presente/ausente y registry vacío.

## Tareas TDD

- [x] RED: tests de los cuatro escenarios (imagen principal, fallback, bloque
      adicional, registry vacío).
- [x] GREEN: implementación mínima del truncado/tooltip y del fallback.
- [x] REFACTOR: extraer fila etiqueta/valor repetida a un helper interno.
- [x] `pnpm exec tsc -b`, `pnpm run lint`, `npx vitest run` en verde.
- [x] `docs/inspect-modal-image.md` y cierre de #188.