# PLAN-004 — Validar la expresión cron en el PUT de /api/update-check/config

- **Issue**: #137 (https://github.com/atareao/alloy/issues/137)
- **Change OpenSpec**: `validate-cron-on-put`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/validate-cron-on-put` desde `develop`; PR a `develop`.

## Objetivo

Rechazar con **400** las expresiones cron inválidas en `PUT /api/update-check/config`, en vez de persistirlas en silencio, con un helper de validación de punto único y propagación del error en la UI.

## Decisiones (operador)

- Cron inválido → **400** con `{"error": msg}`; no se persiste.
- Formato canónico: **solo 5 campos** (`0 6 * * *`); un input de 6 campos → 400.
- Cron **vacío** → 400.

## Problema

- `admin.rs:59` escribe `body.cron` sin validar; `admin.rs:74` deja `next_run_at = null` si es inválido. El scheduler lo ignora con `warn!` (`scheduler.rs:62-65`).
- Sin helper de validación; parseo (prefijo `"0 "` para cron 0.15, 6 campos) duplicado en `admin.rs:42`, `timezone.rs:52`, `scheduler.rs:59`.
- Frontend muestra mensaje genérico sin leer el body del 400 (`ConfigPage.tsx:288-331`).

## Alcance

1. `backend/src/timezone.rs`: `pub fn is_valid_cron(expr: &str) -> bool` (exige 5 campos, parsea `"0 {expr}"`).
2. `backend/src/admin.rs`: `put_update_check_config_h` → `Result<Json<UpdateCheckConfig>, AppError>`; valida `body.cron` antes de mutar; `AppError::BadRequest` (400).
3. `frontend/src/components/ConfigPage.tsx`: `saveUpdateCheck` lee `data.error` del backend y lo muestra en el `Alert` (patrón de `:518-524`).
4. Tests backend (helper + PUT: inválido/vacío/6-campos/válido) y frontend (error del backend mostrado).

## Fuera de alcance (posible follow-up)

- Validación cliente (regex) del cron en el formulario.
- Normalizar inputs de 6 campos.

## Ficheros implicados

- `backend/src/{timezone.rs, admin.rs}`.
- `frontend/src/components/ConfigPage.tsx` (+ `ConfigPage.test.tsx`).

## Plan TDD

### RED
- `is_valid_cron`: 5 campos ⇒ true; vacío/texto/6 campos ⇒ false.
- PUT: inválido/vacío/6-campos ⇒ 400 y settings sin cambios; válido ⇒ 200.
- Frontend: guardar con 400 ⇒ se muestra `data.error`.

### GREEN
- Helper + validación en el PUT + lectura del error en el frontend.

### REFACTOR / verificación
- `cd backend && just check` + `cargo test`.
- `cd frontend && pnpm exec tsc -b` + `pnpm run build` + tests.
- `rg` para confirmar el punto único de validación.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Backend y frontend en verde.
- [ ] `docs/validate-cron-on-put.md`.
- [ ] `openspec archive validate-cron-on-put` + limpieza.
- [ ] PR a `develop` mergeado e issue #137 cerrado.

## Referencias

- #125 / PR #129; follow-ups #130/#131, #132/#133, #135/#136.
- Spec delta: `openspec/changes/validate-cron-on-put/specs/updates/update-check/spec.md`.
- `docs/reconcile-update-defaults.md`; `models.rs:279` (`DEFAULT_UPDATE_CHECK_CRON`), `models.rs:528-551` (`AppError`).
