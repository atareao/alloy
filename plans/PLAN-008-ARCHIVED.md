# PLAN-008 — Editar check_interval_ms en la UI (en segundos)

- **Issue**: #149 (https://github.com/atareao/alloy/issues/149)
- **Change OpenSpec**: `expose-check-interval-ui`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/expose-check-interval-ui` desde `develop`; PR a `develop`.

## Objetivo

Permitir editar `check_interval_ms` (intervalo en ms entre verificaciones de digest de cada contenedor) desde el formulario, en **segundos**, con default/etiqueta desde la API.

## Decisiones (operador)

- **Exponerlo y editarlo en la UI** (el backend ya lo acepta y devuelve).
- Unidad: **segundos** (conversión a ms al enviar).

## Problema

- `check_interval_ms` se acepta en `PUT` y se devuelve en `GET`/`/defaults`, pero la UI no lo muestra ni lo edita; el usuario no puede ajustarlo.
- `interface UpdateCheckConfig` (TS) ni siquiera incluye el campo.

## Alcance (solo frontend)

1. `frontend/src/types.ts`: añadir `check_interval_ms` a `UpdateCheckConfig`.
2. `frontend/src/components/ConfigPage.tsx`:
   - Estado `checkIntervalSecs` (segundos); cargar `data.check_interval_ms / 1000`.
   - `InputNumber` (min 1 s) en el bloque de revisión.
   - Enviar `check_interval_ms: secs * 1000` en el `PUT`.
   - Etiqueta/placeholder desde `defaults.check_interval_ms / 1000`.
3. Tests: cargar (2 s), guardar (5 s ⇒ 5000), etiqueta desde la API.

## Fuera de alcance

- Cambios de backend (ya soporta el campo).
- Validación cliente del cron por rangos (#150).

## Caveats

- La edición es en **segundos enteros**: un valor almacenado no múltiplo de 1000 se redondeará al mostrar/guardar. Anotarlo en la doc.

## Plan TDD

### RED
- Test: config `2000` ⇒ control muestra `2`.
- Test: poner `5` y guardar ⇒ `PUT` envía `5000`.
- Test: etiqueta/placeholder desde `defaults` (en segundos).

### GREEN
- Tipo TS + estado + `InputNumber` + conversión en el `PUT`.

### REFACTOR / verificación
- `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests.

## Verificación de cierre

- [x] Spec aprobado (`.approved`).
- [x] RED → GREEN → REFACTOR.
- [x] Frontend en verde.
- [x] `docs/expose-check-interval-ui.md`.
- [x] `openspec archive expose-check-interval-ui` + limpieza.
- [x] PR a `develop` mergeado e issue #149 cerrado.

## Referencias

- #135 / PR #136; #139 / PR #140; #148 / PR #151.
- Spec delta: `openspec/changes/expose-check-interval-ui/specs/updates/update-check/spec.md`.
- `admin.rs:38,79-80,96`; `models.rs:252-256,375-376`; `types.ts:136-149`.
