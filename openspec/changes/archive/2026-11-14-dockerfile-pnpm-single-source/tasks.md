# Tasks

## 1. RED — Demostrar el guard

- [x] 1.1 Extraer el script del guard y ejecutarlo contra el `Dockerfile` **actual** (con
      `pnpm@11.21.0`): debe **fallar** (literal de pnpm detectado). Guardar la salida literal como
      evidencia de RED.
- [x] 1.2 Banco de casos del guard endurecido: (1) Dockerfile actual → exit 0; (2) hardcode literal
      dejando el comentario → exit 1; (3) deriva borrada pero comentario intacto → exit 1;
      (4) `ARG PNPM_PIN=11.21.0` + `pnpm@${PNPM_PIN}` con la deriva conservada → exit 1.

## 2. GREEN — Derivar y vigilar

- [x] 2.1 `Dockerfile`: stage `frontend-builder` adelanta `COPY frontend/package.json ./` y deriva
      la versión con `node -p` sobre `packageManager`; elimina el literal `pnpm@11.21.0`.
- [x] 2.2 `ci.yml`: paso `Check pnpm version consistency` tras el de Node (falla si el Dockerfile
      fija un literal o si deja de referenciar `packageManager`).

## 3. REFACTOR y verificación

- [x] 3.1 Guard en verde contra el `Dockerfile` nuevo (caso 1) y en rojo en los casos 2-4.
- [x] 3.2 Deriva local:
      `node -p 'require("./frontend/package.json").packageManager.replace(/^pnpm@/, "").split("+")[0]'`
      → `11.21.0` (coincide con la versión instalada hoy); `case` rechaza vacío/`+sha512…`/prefijo
      ajeno.
- [x] 3.3 Build del stage `frontend-builder` válido (`podman build --target frontend-builder`).
- [x] 3.4 YAML válido (`yaml.safe_load`) y `actionlint` sin avisos nuevos.

## 4. Documentación y cierre

- [x] 4.1 `docs/pnpm-single-package-manager.md`, `docs/frontend-ci-verification.md` actualizados;
      retirado el follow-up de `docs/ci-node-24.md`.
- [x] 4.2 Auditar con el revisor de frontend (`react-reviewer`): ronda 1 RECHAZADO (2 mayores) →
      correcciones → ronda 2 APROBADO; cerrados también 3 residuos menores (guard aditivo, prerelease,
      redacción).
- [ ] 4.3 Abrir PR a `develop`, pasar CI y archivar con `openspec archive`.
