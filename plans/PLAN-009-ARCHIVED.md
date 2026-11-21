# PLAN-009 — Endurecer la validación cliente del cron (rangos y sintaxis)

- **Issue**: #150 (https://github.com/atareao/alloy/issues/150)
- **Change OpenSpec**: `harden-cron-client-validation`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/harden-cron-client-validation` desde `develop`; PR a `develop`.

## Objetivo

Que el formulario detecte en cliente crons fuera de rango o con sintaxis inválida (no solo estructural), con error inline y bloqueo del guardado.

## Decisiones (operador)

- Validar **por rangos/sintaxis** (minuto 0–59, hora 0–23, día 1–31, mes 1–12/nombres, día-semana 0–7/nombres), con listas, rangos `a-b`, pasos `/n` (>0) y rangos no invertidos.
- **Sin dependencias** nuevas.
- Sigue **bloqueando** el guardado (error inline).

## Problema

- `isValidCronClient` es solo estructural (5 campos + charset): `99 99 99 99 99`, `30-10 * * * *` o `*/0 * * * *` pasan el pre-chequeo y el usuario solo ve el 400 del backend.

## Alcance (solo frontend)

1. `ConfigPage.tsx`: endurecer `isValidCronClient`/`cronErrorFor` (validación por campo), manteniendo el prefijo de error "expresión cron inválida".
2. `ConfigPage.test.tsx`: fuera de rango/rango invertido/paso cero ⇒ error + PUT no disparado; válidos ⇒ PUT; adaptar el test "backend responde 400" a un cron client-valid con mock 400.

## Fuera de alcance

- Paridad exacta con la crate `cron` (`?`, `L`, `W`; combinaciones día/mes).
- Backend (sigue siendo la autoridad final; sin cambios).

## Caveats

- Si el cliente acepta algo que el backend rechaza, la UI sigue mostrando el mensaje del backend (cobertura preservada vía mock).

## Plan TDD

### RED
- Tests de fuera de rango, rango invertido, paso cero (error + sin PUT).
- Tests de válidos (`*/15 * * * *`, `0 6 * * MON`, `0 6 1 JAN *`, `0 6 * * 1-5`).

### GREEN
- Validación por campo (rangos, listas, rangos, pasos, nombres).

### REFACTOR / verificación
- `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests.

## Verificación de cierre

- [x] Spec aprobado (`.approved`).
- [x] RED → GREEN → REFACTOR.
- [x] Frontend en verde.
- [x] `docs/harden-cron-client-validation.md`.
- [x] `openspec archive harden-cron-client-validation` + limpieza.
- [x] PR a `develop` mergeado e issue #150 cerrado.

## Referencias

- #137 / PR #138 (`is_valid_cron`, crate `cron` 0.15); #141 / PR #142.
- Spec delta: `openspec/changes/harden-cron-client-validation/specs/updates/update-check/spec.md`.
- `backend/src/timezone.rs:67-81`; `ConfigPage.tsx` (`isValidCronClient`, `cronErrorFor`).
