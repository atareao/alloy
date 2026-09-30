# Design

## Context

El Dockerfile copia `frontend/` completo tras instalar dependencias:

```dockerfile
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./          # ← incluye node_modules local si no está en .dockerignore
RUN pnpm run build
```

`.dockerignore` excluye `frontend/dist/` y `frontend/package-lock.json`, pero no
`frontend/node_modules/`. En local, ese `node_modules` (de otra versión de pnpm) pisa el
instalado y `pnpm run build` aborta con `ERR_PNPM_ABORTED_REMOVE_MODULES_DIR_NO_TTY`.

## Goals / Non-Goals

**Goals:**
- Que `just build` funcione en local con `node_modules` presente.
- Mantener el build de CI idéntico.

**Non-Goals:**
- Cambiar el gestor de paquetes o la versión de pnpm.
- Optimizar capas del Dockerfile.

## Decisions

### D1: Excluir `frontend/node_modules/` en `.dockerignore`
Es la causa raíz: evita copiar dependencias locales. Alternativa descartada: borrar
`node_modules` antes del build — frágil y manual.

### D2: `ENV CI=true` en el stage frontend
Defensa en profundidad: pnpm nunca espera TTY. Alternativa descartada: `--config.confirmModulesPurge=false`
— más específico pero no cubre otras interacciones.

## Risks / Trade-offs

- **Riesgo**: si en el futuro se necesita `node_modules` en el contexto, habría que
  revisarlo. No aplica: el Dockerfile lo instala dentro de la imagen.
