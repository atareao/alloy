# Tasks

## 1. Verificación del estado previo (RED)

- [x] 1.1 Confirmar el estado indeseado: `ls frontend/package-lock.json frontend/pnpm-lock.yaml` lista **dos** lockfiles.
- [x] 1.2 Confirmar la contradicción de contrato: `rg -n "npm audit|package-lock" openspec/specs/security/dependency-hygiene/spec.md` devuelve coincidencias.
- [x] 1.3 Confirmar que el build ya es pnpm: `rg -n "pnpm" Dockerfile` devuelve el stage `frontend-builder`.
- [x] 1.4 Capturar baseline en verde: `cd frontend && pnpm install --frozen-lockfile && pnpm run test && pnpm run build`.

## 2. Unificación del lockfile

- [x] 2.1 Eliminar `frontend/package-lock.json`.
- [x] 2.2 Añadir `"packageManager": "pnpm@11.21.0"` a `frontend/package.json`, alineado con el pin del `Dockerfile`.
- [x] 2.3 Retirar de `.dockerignore` la regla que excluye `frontend/package-lock.json`, conservando `frontend/node_modules/`.
- [x] 2.4 Verificar coherencia del pin: la versión mayor de `packageManager` coincide con la del stage `frontend-builder`.

## 3. Documentación npm → pnpm

- [x] 3.1 `README.md`: sustituir las 3 invocaciones npm del frontend por pnpm.
- [x] 3.2 `AGENTS.md`: sustituir `npm run build`, `npx tsc -b` y `npm run lint` por sus equivalentes pnpm.
- [x] 3.3 `docs/security-dependencies.md`: actualizar la descripción de lockfiles, los comandos de auditoría y retirar el follow-up ya resuelto.

## 4. Verificación (GREEN)

- [x] 4.1 `cd frontend && pnpm install --frozen-lockfile` completa sin error y sin modificar `pnpm-lock.yaml`.
- [x] 4.2 `cd frontend && pnpm run test` — 72 tests en verde (8 ficheros).
- [x] 4.3 `cd frontend && pnpm exec tsc -b` sin errores de tipo.
- [x] 4.4 `cd frontend && pnpm run build` completa.
- [x] 4.5 `cd frontend && pnpm run lint` sin errores.
- [x] 4.6 `cd frontend && pnpm audit` sin vulnerabilidades abiertas.
- [x] 4.7 `cd backend && cargo clippy -- -D warnings` limpio y tests **unitarios** en verde (300 pasan). Nota: los 4 `containers::tests::test_integration_*` fallan por exigir un daemon Docker con `alloy`/`oxinbox` vivos — preexistente y fuera de alcance.
- [x] 4.8 Confirmar que `frontend/` contiene exactamente un lockfile.

## 5. Cierre

- [ ] 5.1 Actualizar el spec vivo `openspec/specs/security/dependency-hygiene/spec.md` vía `openspec archive pnpm-single-package-manager --yes`.
- [ ] 5.2 Verificar que `openspec/changes/pnpm-single-package-manager/` fue eliminado; si no, `rm -rf`.
- [ ] 5.3 Verificar tras el merge que Dependabot sigue versionando el ecosistema `npm` de `/frontend` contra `pnpm-lock.yaml` (ver `docs/security-dependencies.md`).
- [x] 5.4 Documentar el cambio en `docs/` y registrar el follow-up del job de frontend en CI.
- [ ] 5.5 Corregir a mano el `## Purpose` del spec vivo `security/dependency-hygiene`: el archive no lo reescribe y aún menciona `frontend/package-lock.json`.
