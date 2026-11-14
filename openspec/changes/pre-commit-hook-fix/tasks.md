# Tasks

## 1. RED — Reproducir el bug

- [x] 1.1 Reproducir la expresión buggy: con 0 coincidencias, `INCOMPLETE=$(grep -c '\[ \]' f || echo 0)`
      da `"0\n0"` y `[ "$INCOMPLETE" -gt 0 ]` falla con *"se esperaba un entero"*. Guardar salida.
- [x] 1.2 Comprobar que el hook **no está versionado** (`git ls-files .git/hooks/pre-commit` vacío).

## 2. GREEN — Corregir y versionar

- [x] 2.1 Crear `.githooks/pre-commit` (copia canónica) con el contador corregido.
- [x] 2.2 `.justfile`: receta `install-hooks` (`git config core.hooksPath .githooks`) + aviso.
- [x] 2.3 `docs/pre-commit-hook.md` + mención en `README.md`.

## 3. REFACTOR y verificación

- [x] 3.1 `bash -n .githooks/pre-commit` (sintaxis) OK.
- [x] 3.2 Con 0 tareas incompletas: el hook no imprime ningún error de `[` y sale 0.
- [x] 3.3 Con N>0 tareas incompletas: avisa con N y sale 0 (no bloquea por el aviso).
- [x] 3.4 Sin proposal activo + `.rs` staged: el hook sale != 0.

## 4. Documentación y cierre

- [x] 4.1 Verificar `just install-hooks` en un repo de prueba (o `git config` local reversible).
- [ ] 4.2 Auditar, abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
