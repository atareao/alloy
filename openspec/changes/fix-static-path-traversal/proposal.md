# Proposal

## Why

`frontend_handler` (`backend/src/auth.rs:511`) construye la ruta de disco concatenando el URI crudo sin
normalizar: `format!("dist/{}", file_path)`. Es el `.fallback` del router (`backend/src/main.rs:310`) y el
middleware deja pasar todo lo que no empieza por `/api/` (`auth.rs:320-321`). Una petición cruda con `..`
(p. ej. `GET /../../proc/self/environ`) escapa de `dist/` y permite leer ficheros del contenedor, incluida
la **fuga de variables de entorno con secretos** (OIDC, firma de sesión).

## What Changes

- **Backend** (`src/auth.rs`): el servido de estáticos pasa a resolverse de forma **segura**:
  - nueva función pura y testeable `safe_static_path(root, request_path) -> Option<PathBuf>` que
    canonicaliza y exige que el resultado esté **dentro** de `dist/` (rechaza `..`, rutas absolutas y
    equivalentes percent-encoded);
  - `frontend_handler` la usa: si devuelve un fichero existente dentro de `dist/`, lo sirve; en cualquier
    otro caso aplica el *fallback* SPA (`dist/index.html`), preservando el enrutado del cliente.
- **Tests**: unitarios del helper (traversal `..`, `%2e%2e`, absolutos, ruta válida) + integración del
  handler (asset servido; traversal no sirve el fichero externo).

## Capabilities

### New Capabilities

- `security/static-serving`: el servido de estáticos queda confinado a `dist/`.

### Modified Capabilities

- Ninguna.

## Impact

- **Backend**: `src/auth.rs` (+ tests). **Sin cambios de API ni de datos.**
- **Frontend**: ninguno (mismo comportamiento para rutas legítimas y fallback SPA).
- **Compatibilidad**: el comportamiento observable para el cliente legítimo no cambia (assets y SPA siguen
  funcionando).
