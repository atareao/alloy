# PLAN-028 — Deduplicar @ant-design/cssinjs

- Issue: #190 (follow-up de #163 fase 1)
- Tipo: dependencias frontend
- Change OpenSpec: `openspec/changes/dedupe-antd-cssinjs/`
- Doc final: `docs/dedupe-antd-cssinjs.md`

## Objetivo

Una sola copia de `@ant-design/cssinjs` en el árbol de dependencias. Hoy
`pnpm-lock.yaml` resuelve 2.1.2 (antd 6.7.1, rango `~2.1.2`) y 2.2.0
(`@ant-design/cssinjs-utils`, rango `^2.1.2`).

## Decisión

Forzar `@ant-design/cssinjs@2.1.2` con `overrides` en `frontend/pnpm-workspace.yaml`:
satisface el rango de antd (`~2.1.2`) y el de `cssinjs-utils` (`^2.1.2`), y evita
adoptar 2.2.0, que antd excluye explícitamente. Si 2.1.2 rompe el build o los
tests, la alternativa es 2.2.0 y se documenta el motivo.

pnpm ≥11 no lee el campo `pnpm` de `package.json` (lo ignora con warning): los
overrides se declaran en `pnpm-workspace.yaml`. El `Dockerfile` debe copiar ese
fichero al contexto de build para que la instalación en la imagen aplique el
mismo override (fuera del alcance de este subagente).

## Tareas

- [x] RED: comprobar que `pnpm why @ant-design/cssinjs` muestra dos versiones.
- [x] Añadir el override en `frontend/pnpm-workspace.yaml` (pnpm 11) y regenerar `pnpm-lock.yaml` (sin `--frozen-lockfile`).
- [x] GREEN: una sola versión en `pnpm why` y en el lock.
- [x] `pnpm exec tsc -b`, `pnpm run lint`, `npx vitest run`, `pnpm run build` → 0 (209 tests).
- [x] Verificar que el bundle no arrastra dos copias del runtime de estilos (3157 → 3125 módulos, −7,62 kB).
- [x] `docs/dedupe-antd-cssinjs.md` y `docs/frontend-deps.md` actualizados. Cerrar #190 pendiente (lo hace el orquestador).