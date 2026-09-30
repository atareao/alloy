# Proposal

## Why

El build local de la imagen (`just build` → podman) falla en el stage de frontend:

```
[ERR_PNPM_ABORTED_REMOVE_MODULES_DIR_NO_TTY] Aborted removal of modules directory due to no TTY
```

Causa: `.dockerignore` **no excluye `frontend/node_modules/`**. El `COPY frontend/ ./` del
Dockerfile copia el `node_modules` local (generado con otra versión de pnpm) dentro de la
imagen, pisando el que instaló `pnpm install --frozen-lockfile`. Al ejecutar
`pnpm run build`, pnpm detecta el `node_modules` incompatible e intenta purgarlo, pero sin
TTY aborta. En CI no ocurre porque el checkout no contiene `node_modules`.

## What Changes

- Añadir `frontend/node_modules/` a `.dockerignore` para que el contexto de build no
  incluya dependencias locales.
- Añadir `ENV CI=true` al stage `frontend-builder` del Dockerfile para que pnpm sea
  no-interactivo (defensa en profundidad).

## Capabilities

### New Capabilities
- `build/docker-context`: el contexto de build excluye artefactos locales
  (`node_modules`) y el stage de frontend es no-interactivo.

### Modified Capabilities
<!-- Ninguna. -->

## Impact

- `.dockerignore`: nueva entrada `frontend/node_modules/`.
- `Dockerfile`: `ENV CI=true` en el stage `frontend-builder`.
- Sin cambios de código Rust/TS. Sin cambios de runtime.
