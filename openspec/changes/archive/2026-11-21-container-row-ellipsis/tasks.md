# Tasks

## TDD Checklist

### Tests

- [x] Actualizar `ContainerRow.test.tsx`: el test de truncado móvil ya no debe esperar `slice` manual; debe comprobar que el nombre completo está en el DOM y que el truncado es por elipsis.
- [x] Añadir test de tooltip/valor completo en nombre y estado.
- [x] Añadir test de que el badge de actualización y el botón de expansión siguen visibles con un nombre muy largo.

### Implementación

- [x] Quitar el `slice` manual de nombre y estado en `ContainerRow.tsx` (móvil y escritorio).
- [x] Usar elipsis real con tooltip en nombre y estado.
- [x] Fijar `minWidth: 0` / `flex` en los contenedores de nombre y estado; `flexShrink: 0` en los elementos no truncables.
- [x] Sustituir `progress.checking.slice(0, 20)` por elipsis real con tooltip.
- [x] Revisar el ancho del nombre del stack en `DashboardPage.tsx` si el truncado no es efectivo.

### Verificación

- [x] `pnpm run test` → 0 (≥174 tests).
- [x] `pnpm exec tsc -b` → 0.
- [x] `pnpm run lint` → 0.
- [x] `pnpm run build` → 0.
- [ ] Commit con gitmoji (`style` o `feat`) y PR a `develop`.
