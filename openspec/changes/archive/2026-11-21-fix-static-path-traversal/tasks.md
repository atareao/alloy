# Tasks — fix-static-path-traversal

## 1. RED (tests que fallan)

- [x] 1.1 Unit: `safe_static_path` devuelve `None` para `../../Cargo.toml`, `/%2e%2e/%2e%2e/etc/passwd`,
      `/etc/passwd` (absoluto) y rutas con `\0`. → 6 tests en `auth::tests`.
- [x] 1.2 Unit: `safe_static_path` devuelve `Some` (dentro de `dist/`) para `/assets/app.js`. Para `/` y una
      ruta de cliente inexistente devuelve `None` (no son ficheros): el *fallback* SPA se resuelve en
      `serve_static`. Documentado en el doc-comment de `safe_static_path`.
- [x] 1.3 Integración: `serve_static` (lógica extraída de `frontend_handler`, que queda como envoltorio
      fino) sirve `assets/app.js` con su `content-type`; y ante `/../<fichero externo>` NO devuelve su
      contenido (cae al *fallback* SPA). Se testea contra un `root` temporal para no depender de `dist/`.

## 2. GREEN

- [x] 2.1 `src/auth.rs`: añadir `safe_static_path` (canonicaliza + exige prefijo `dist/`) y usarla en
      `frontend_handler`; mantener el *fallback* SPA para rutas legítimas.

## 3. REFACTOR / verificación

- [x] 3.1 `cd backend && just check` + `cargo test` en verde. (`just check` limpio; 445 passed / 0 failed,
      12 ignored — 432 base + 13 nuevos.)
- [x] 3.2 Docs: `docs/fix-static-path-traversal.md`.

## 4. Archive

- [x] 4.1 `openspec archive fix-static-path-traversal` tras merge.
