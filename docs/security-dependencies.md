# Remediación de dependencias vulnerables (Dependabot)

## Qué es

Actualización de dependencias **transitivas** a sus versiones parcheadas para cerrar las 44
alertas de seguridad de Dependabot abiertas en `atareao/alloy`. Sin cambios de código de
aplicación ni de comportamiento.

## Ficheros implicados

- `frontend/package-lock.json` — lockfile de `npm` (uso local).
- `frontend/pnpm-lock.yaml` — lockfile de `pnpm` (stage `frontend-builder` del `Dockerfile`).
- `backend/Cargo.lock` — lockfile de `cargo`.

## Cómo funciona

Se regeneran los lockfiles con las versiones parcheadas dentro de los rangos ya declarados en
`package.json` / `Cargo.toml`. No se toca ningún `.rs` / `.tsx` / `.ts`.

Paquetes actualizados y versión mínima parcheada:

| Paquete | Antes | Después | Introducido por |
|---|---|---|---|
| undici | 7.30.0 | 7.30.2 | `jsdom` (dev/test) |
| vite | 8.1.5 | 8.3.6 | devDependency directa |
| vitest | 4.1.11 | 4.1.12 | devDependency directa |
| @vitest/mocker | 4.1.11 | 4.1.12 | `vitest` |
| postcss | 8.5.15 | 8.5.32 | `vite` |
| source-map-js | 1.2.1 | 1.2.2 | `postcss` / `css-tree` |
| rustls | 0.23.42 | 0.23.46 | `bollard` / `reqwest` |

`launch-editor` (avisos atribuidos a `vite`) desaparece: `vite` 8.3.6 ya no lo usa.

## Configuración

Ninguna. Sin variables de entorno ni cambios de despliegue.

## Cómo verificar

```bash
cd frontend && npm audit                    # 0 vulnerabilities
cd frontend && npx tsc -b && npx vitest run && npm run build
cd backend  && cargo test && cargo clippy -- -D warnings
gh api repos/atareao/alloy/dependabot/alerts --jq '[.[]|select(.state=="open")]|length'  # 0
```

## Decisiones

- Bumps **solo** en lockfiles para minimizar el diff; no se ampliaron rangos en `package.json`.
- `pnpm-lock.yaml` se regeneró con `pnpm@11.21.0` (la versión del `Dockerfile`) y se validó
  con `pnpm install --frozen-lockfile`; los *specifiers* del importer se mantuvieron en los
  valores del manifest para no romper el build Docker.
- Se conserva `@testing-library/jest-dom` en `6.9.1` (la `6.10.0` está deprecada por breaking
  changes).

## Follow-ups

- Dependabot solo cierra las alertas cuando el fix llega a la rama por defecto (`main`).
- Valorar unificar en un único gestor de paquetes (hoy conviven `npm` y `pnpm`).
