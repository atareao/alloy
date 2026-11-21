# PLAN-010 — Confinar el servido de estáticos a `dist/` (fix path traversal)

- **Issue**: #164 (https://github.com/atareao/alloy/issues/164)
- **Change OpenSpec**: `fix-static-path-traversal`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `fix/static-path-traversal` desde `develop`; PR a `develop`.

## Objetivo

Eliminar la vulnerabilidad de path traversal en el handler de estáticos, confinando el servido a `dist/` y preservando el fallback SPA para rutas legítimas.

## Problema

- `frontend_handler` (`backend/src/auth.rs:511-518`) hace `format!("dist/{}", uri.path().trim_start_matches('/'))` sin normalizar.
- Es el `.fallback` del router (`backend/src/main.rs:310`); el middleware deja pasar todo lo que no empieza por `/api/` (`auth.rs:320-321`).
- `GET /../../proc/self/environ` → fuga de secretos (env con OIDC/firma de sesión) y otros ficheros del contenedor.

## Alcance (solo backend)

1. Nueva función pura `safe_static_path(root, request_path) -> Option<PathBuf>`: canonicaliza y exige que el resultado quede **dentro** de `dist/`; rechaza `..`, rutas absolutas, `\0` y equivalentes percent-encoded.
2. `frontend_handler` la usa: sirve solo ficheros existentes dentro de `dist/`; en cualquier otro caso, fallback SPA (`dist/index.html`).
3. Tests: unitarios del helper (traversal, absolutos, válidos) + integración del handler.

## Fuera de alcance

- Cambios de API/datos, frontend, o los demás refuerzos de seguridad del issue #159.

## Caveats

- El fallback SPA debe seguir sirviendo `dist/index.html` para rutas de cliente desconocidas.
- `uri.path()` no viene percent-decoded: hay que cubrir el caso `%2e%2e` explícitamente.

## Plan TDD

### RED
- Unit: `safe_static_path` → `None` para `../../Cargo.toml`, `/%2e%2e/%2e%2e/etc/passwd`, `/etc/passwd`, `\0`.
- Unit: `Some` dentro de `dist/` para `/`, `/assets/app.js`, y ruta de cliente inexistente (fallback).
- Integración: sirve asset; traversal no filtra el fichero externo.

### GREEN
- `safe_static_path` + uso en `frontend_handler`.

### REFACTOR / verificación
- `cd backend && just check` + `cargo test`.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Backend en verde (`cargo test`, `just check`).
- [ ] `docs/fix-static-path-traversal.md`.
- [ ] `openspec archive fix-static-path-traversal` + limpieza.
- [ ] PR a `develop` mergeado e issue #164 cerrado.

## Referencias

- Spec delta: `openspec/changes/fix-static-path-traversal/specs/security/static-serving/spec.md`.
- `backend/src/auth.rs:305-322, 511-550`; `backend/src/main.rs:310`.
- Backlog relacionado: #159 (refuerzos de seguridad backend).
