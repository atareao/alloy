# Tasks

## Checklist

### Actualización

- [x] Ejecutar `pnpm update` en `frontend/` (dentro de rangos, sin majors).
- [x] Ajustar `package.json` solo si algún rango impide llegar a la versión objetivo.
- [x] Confirmar con `pnpm outdated` que no quedan menores/parches pendientes.

### Verificación

- [x] `pnpm exec tsc -b` → 0.
- [x] `pnpm run test` → 0 (≥174 tests).
- [x] `pnpm run lint` → 0.
- [x] `pnpm run build` → 0.
- [x] `pnpm install --frozen-lockfile` → 0 sin modificar el lockfile.
- [x] Commit con gitmoji (`deps`) y PR a `develop`.
