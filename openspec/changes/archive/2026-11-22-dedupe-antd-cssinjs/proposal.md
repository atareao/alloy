# Proposal

## Why

Issue #190 (follow-up de #163 fase 1, con antd 6.7.1). El `frontend/pnpm-lock.yaml` resuelve **dos copias** de `@ant-design/cssinjs`:

- `@ant-design/cssinjs@2.1.2`: la que usa `antd` 6.7.1, que declara el rango `~2.1.2`.
- `@ant-design/cssinjs@2.2.0`: la que arrastra `@ant-design/cssinjs-utils@2.1.2`, que declara `^2.1.2`.

Son dos copias del runtime de estilos en el bundle. Duplican código y pueden producir dos cachés de estilos distintas.

## What Changes

- Añadir `overrides` en `frontend/pnpm-workspace.yaml` fijando `@ant-design/cssinjs` a **`2.1.2`**. pnpm ≥11 ignora el campo `pnpm` de `package.json`, así que los overrides se declaran en `pnpm-workspace.yaml`.
- Regenerar `frontend/pnpm-lock.yaml` para que quede una única versión `@ant-design/cssinjs`.
- Documentar la decisión, la verificación y la alternativa en `docs/dedupe-antd-cssinjs.md` y actualizar `docs/frontend-deps.md`.
- Verificar que el build y los tests siguen en verde.

## Decisión

Fijar `2.1.2`:

- Satisfaga los dos rangos declarados: `~2.1.2` (antd) y `^2.1.2` (`@ant-design/cssinjs-utils`).
- No adopta `2.2.0`, que antd excluye explícitamente con `~`.

Alternativa documentada: si el build o los tests fallan con `2.1.2`, fijar `2.2.0` (compatible con `^2.1.2` de utils) y revisar que antd no dependa de APIs de `2.1.x`. Se elige `2.1.2` primero porque es la versión que antd ya espera.

## Capabilities

### New Capabilities

- `frontend/deps-dedupe`: una única versión resuelta de `@ant-design/cssinjs` en el lock del frontend y verificación reproducible.

### Modified Capabilities

Ninguna.

## Impact

- `frontend/pnpm-workspace.yaml` (`overrides`; en pnpm ≥11 el campo `pnpm` de `package.json` se ignora).
- `frontend/pnpm-lock.yaml` (regenerado con `pnpm install`).
- `Dockerfile` (debe copiar `frontend/pnpm-workspace.yaml` al contexto de build; lo aplica el subagente de Docker).
- `docs/dedupe-antd-cssinjs.md`, `docs/frontend-deps.md`.

### Fuera de alcance

- Subir o bajar `antd`.
- Tocar otros duplicados del lock.

Issue: #190.