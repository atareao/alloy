# Tasks — fix-static-path-traversal

## 1. RED (tests que fallan)

- [ ] 1.1 Unit: `safe_static_path` devuelve `None` para `../../Cargo.toml`, `/%2e%2e/%2e%2e/etc/passwd`,
      `/etc/passwd` (absoluto) y rutas con `\0`.
- [ ] 1.2 Unit: `safe_static_path` devuelve `Some` (dentro de `dist/`) para `/`, `/assets/app.js` y una
      ruta de cliente inexistente (para el fallback).
- [ ] 1.3 Integración: `frontend_handler` sirve `dist/assets/app.js`; y ante `/../<fichero externo>` NO
      devuelve su contenido.

## 2. GREEN

- [ ] 2.1 `src/auth.rs`: añadir `safe_static_path` (canonicaliza + exige prefijo `dist/`) y usarla en
      `frontend_handler`; mantener el *fallback* SPA para rutas legítimas.

## 3. REFACTOR / verificación

- [ ] 3.1 `cd backend && just check` + `cargo test` en verde.
- [ ] 3.2 Docs: `docs/fix-static-path-traversal.md`.

## 4. Archive

- [ ] 4.1 `openspec archive fix-static-path-traversal` tras merge.
