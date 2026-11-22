# Tasks

## TDD Checklist

### RED

- [x] Registrar en `docs/dedupe-antd-cssinjs.md` la comprobación inicial: `pnpm why @ant-design/cssinjs` muestra dos versiones (`2.1.2` y `2.2.0`).
- [x] Documentar la entrada duplicada en `frontend/pnpm-lock.yaml` (`@ant-design/cssinjs@2.1.2` y `@ant-design/cssinjs@2.2.0`) como estado de partida.

### GREEN

- [x] Añadir el override `@ant-design/cssinjs: 2.1.2` en `frontend/pnpm-workspace.yaml` (pnpm 11 ignora el campo `pnpm` de `package.json`; `pnpm.overrides` ya no se lee).
- [x] Regenerar `frontend/pnpm-lock.yaml` con `pnpm install --no-frozen-lockfile`.
- [x] Confirmar que `pnpm why @ant-design/cssinjs` muestra una sola versión.
- [x] Ejecutar `pnpm exec tsc -b`, `pnpm run lint`, `npx vitest run` y `pnpm run build` en verde (209 tests).

### REFACTOR

- [x] Si el build falla con `2.1.2`, probar la alternativa `2.2.0` (documentada en el proposal) y anotar el resultado. → No aplica: `2.1.2` pasa todo; **no se usa 2.2.0**.
- [x] Revisar que no se hayan movido otros duplicados ni la versión de `antd`.

### Verificación

- [x] Una única entrada `@ant-design/cssinjs@` en el lock.
- [x] `pnpm why @ant-design/cssinjs` → una sola versión.
- [x] `pnpm exec tsc -b`, `pnpm run lint`, `npx vitest run`, `pnpm run build` → 0.
- [x] Documentar en `docs/dedupe-antd-cssinjs.md` y actualizar `docs/frontend-deps.md`.
- [ ] Cerrar el issue #190 con referencia al change.