# Actualización de dependencias del frontend

## Qué es

Actualización de las dependencias del frontend (#163) en dos fases:

- **Fase 1 (hecha):** parches y menores, sin cruzar ningún major. Change OpenSpec `deps-minor`, plan `PLAN-020-ARCHIVED.md`.
- **Fase 2 (hecha):** majors (vitest 5, jsdom 31, TypeScript 7, jest-dom 8). Change OpenSpec `deps-major`, plan `PLAN-021.md`.

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
| `vitest` | 5.1.1 |
| `jsdom` | 31.0.1 |
| `typescript` | 7.0.2 |
| `@testing-library/jest-dom` | 8.0.0 |
| `@testing-library/dom` | `^10.4.2` (devDependency nueva) |
| `@types/node` | `^24.13.2` (resuelve 24.13.3) |

`engines.node` sube a `^24.15`.

## Decisiones

- **`@testing-library/jest-dom` subido a 8.0.0:** en la fase 1 estaba capado a la serie 6 (la 6.10.0 estaba deprecada); la fase 2 cruza a la serie 8.
- **`@testing-library/dom ^10.4.2` como devDependency:** dependencia peer que jest-dom 8 / testing-library exigen declarar explícitamente.
- **`frontend/src/test-setup.ts` → `import "@testing-library/jest-dom/vitest"`:** jest-dom 8 ya no amplía los matchers globalmente y vitest 5 dejó de heredar de `jest.Matchers`, lo que provocaba 41 errores `TS2339` (propiedades como `toBeInTheDocument` no existían). El import del entrypoint `/vitest` registra los matchers con los tipos correctos.
- **`@types/node` en `^24.13.2` (resuelve 24.13.3):** coherencia con `engines.node ^24` y el Dockerfile `node:24`. No se sube a 26.
- **`oxlint` 1.74 → 1.94:** activa 5 warnings preexistentes de `react(set-state-in-effect)` (`TelegramCard.tsx:42`, `ValetCard.tsx:41`, `MatrixCard.tsx:42`, `DashboardPage.tsx:199`, `App.tsx:326`). No bloquean, pero **no añadir `--deny-warnings`** hasta resolverlos.
- **antd 6.7.1:** introduce un duplicado upstream de `@ant-design/cssinjs` (2.1.2 + 2.2.0). **Ya deduplicado en #190** (plan `PLAN-028`): `frontend/pnpm-workspace.yaml` fuerza `2.1.2` con `overrides`. En pnpm 11 los overrides viven en `pnpm-workspace.yaml`, no en el campo `pnpm` de `package.json`. Detalle en `docs/dedupe-antd-cssinjs.md`.

## Verificación

- 174 tests OK; `tsc -b`, lint y build con código 0.
- `pnpm install --frozen-lockfile` OK (lockfile sincronizado).

## Follow-ups

- Resolver los 5 warnings de `react(set-state-in-effect)` antes de endurecer el lint con `--deny-warnings`.
