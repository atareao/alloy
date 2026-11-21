# Actualización de dependencias del frontend

## Qué es

Actualización de las dependencias del frontend (#163) en dos fases:

- **Fase 1 (hecha):** parches y menores, sin cruzar ningún major. Change OpenSpec `deps-minor`, plan `PLAN-020-ARCHIVED.md`.
- **Fase 2 (pendiente):** majors (vitest 5, jsdom 31, TypeScript 7, jest-dom 8), a abordar en un change propio (`deps-major`).

## Ficheros implicados

- `frontend/package.json`
- `frontend/pnpm-lock.yaml`

## Cómo funciona (versiones objetivo)

Versiones resultantes tras `pnpm update`:

| Dependencia | Versión |
|---|---|
| `antd` | 6.7.1 |
| `react` / `react-dom` | 19.3.1 |
| `vite` | 8.3.8 |
| `oxlint` | 1.94.0 |
| `@testing-library/jest-dom` | `~6.9.1` |
| `@types/node` | `^24.13.2` (resuelve 24.13.3) |

Se mantienen en su serie mayor actual: `vitest` 4, `jsdom` 29, `typescript` 6.0, `@testing-library/jest-dom` 6.

## Decisiones

- **`@testing-library/jest-dom` capado a `~6.9.1`:** 6.10.0 está deprecado ("incorrect minor release with breaking changes"), así que no se sube.
- **`@types/node` en `^24.13.2` (resuelve 24.13.3):** coherencia con `engines.node ^24` y el Dockerfile `node:24`. No se sube a 26.
- **`oxlint` 1.74 → 1.94:** activa 5 warnings preexistentes de `react(set-state-in-effect)` (`TelegramCard.tsx:42`, `ValetCard.tsx:41`, `MatrixCard.tsx:42`, `DashboardPage.tsx:199`, `App.tsx:326`). No bloquean, pero **no añadir `--deny-warnings`** hasta resolverlos.
- **antd 6.7.1:** introduce un duplicado upstream de `@ant-design/cssinjs` (2.1.2 + 2.2.0). No forzar con `pnpm.overrides`.

## Verificación

- 174 tests OK; `tsc -b`, lint y build con código 0.
- `pnpm install --frozen-lockfile` OK (lockfile sincronizado).

## Follow-ups

- **Fase 2:** subir majors pendientes (vitest 5, jsdom 31, TypeScript 7, jest-dom 8) en un change nuevo.
- Resolver los 5 warnings de `react(set-state-in-effect)` antes de endurecer el lint con `--deny-warnings`.
