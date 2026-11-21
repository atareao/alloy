# fix-static-path-traversal

Issue: #164 (hallazgo de la revisión de deuda técnica).

## Qué es

Cierra una vulnerabilidad de **path traversal** en el servido de estáticos del frontend. El handler
(`auth.rs`) construía la ruta de disco con el URI crudo sin normalizar, de modo que una petición con `..`
podía escapar de `dist/` y leer ficheros del contenedor (incluidas variables de entorno con secretos).

## Problema (antes)

- `frontend_handler` — `backend/src/auth.rs` — hacía `format!("dist/{}", uri.path().trim_start_matches('/'))`
  sin normalizar.
- Es el `.fallback` del router (`backend/src/main.rs`), y el middleware deja pasar todo lo que no empieza
  por `/api/` (`auth.rs`).
- `GET /../../proc/self/environ` (socket crudo; los navegadores normalizan) escapaba de `dist/`.

## Cómo funciona (después)

- **`safe_static_path(root, request_path) -> Option<PathBuf>`** (función pura): canonicaliza `root` y el
  candidato, y solo devuelve la ruta si el resultado **queda dentro de `root`** y es un fichero. Rechaza
  `..`, rutas absolutas, `\0` y equivalentes percent-encoded (el `%2e%2e` literal no es `..` en el FS, así
  que el `canonicalize` + comprobación de prefijo lo cubre).
- **`serve_static(root, request_path)`**: resuelve con `safe_static_path`; si hay fichero, lo sirve con su
  `content-type`; en cualquier otro caso aplica el **fallback SPA** (`root/index.html`).
- **`frontend_handler`** = `serve_static(Path::new("dist"), path)` (ahora fino y testeable).

El comportamiento observable para el cliente legítimo **no cambia**: assets y rutas SPA siguen sirviéndose.

## Ficheros implicados

- `backend/src/auth.rs`: `safe_static_path`, `serve_static`, `frontend_handler` (+ tests).

## Cómo habilitar / desplegar

Sin acción: el fix viaja con la imagen. No hay variables nuevas.

## Verificación

- Tests unitarios del helper (traversal `..`, `%2e%2e`, absolutos, `\0`, válidos) + test del handler con
  `tempdir` (asset servido; traversal **no** filtra el fichero externo).
- `cd backend && just check` limpio y `cargo test` en verde.

## Caveats / follow-ups

- Mitigable también en el proxy (normalización de `..`), pero el fix es en la app y no depende de eso.
- Otros refuerzos de seguridad del backend (cookie `Secure`, headers, token por query, `unwrap` en el pool):
  issue #159.
