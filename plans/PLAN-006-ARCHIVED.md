# PLAN-006 — Validación cliente del cron y normalización de 6 campos

- **Issue**: #141 (https://github.com/atareao/alloy/issues/141)
- **Change OpenSpec**: `validate-cron-client-ui`
- **Estado**: propuesta SDD creada — PENDIENTE de aprobación del spec
- **Base gitflow**: rama `feature/validate-cron-client-ui` desde `develop`; PR a `develop`.

## Objetivo

Que el formulario valide el cron **antes** de enviarlo (error inline + bloqueo del guardado) y **normalice** un cron de 6 campos a 5. Solo frontend.

## Decisiones (operador)

- Normalizar **6→5 en el frontend** (quitar el campo de segundos); backend sigue estricto en 5.
- Validación cliente **ligera** (regex/estructura), **sin dependencias nuevas**.
- **Bloquear el guardado + error inline** si el cron no pasa la validación cliente.

## Problema

- El campo cron es un `<Input>` plano (`ConfigPage.tsx:967-971`) sin validación; el error solo aparece tras un PUT que devuelve 400.
- Un cron pegado con 6 campos (segundos) se rechaza con 400 pese a ser clara su intención.

## Alcance (solo frontend)

1. `ConfigPage.tsx`:
   - `normalizeCron(expr)`: trim; 6 campos ⇒ quita el primero (segundos).
   - `isValidCronClient(expr)`: 5 campos + charset permitido (dígitos y `* , - /`).
   - Estado `cronError` + error inline bajo el campo.
   - Normalizar onBlur y antes de enviar; `saveUpdateCheck` bloquea el PUT si inválido.
2. Tests: normalización 6→5; malformado ⇒ error inline + PUT no disparado; válido ⇒ sin error + PUT.

## Fuera de alcance

- Validación profunda de rangos (minuto/hora/día); el backend decide.
- Cambios de backend (sigue estricto en 5 campos).

## Ficheros implicados

- `frontend/src/components/ConfigPage.tsx` (+ `ConfigPage.test.tsx`).

## Plan TDD

### RED
- Test 6 campos ⇒ normaliza a 5 y el PUT envía 5.
- Test cron malformado ⇒ error inline y PUT no disparado.
- Test cron válido 5 campos ⇒ sin error y PUT disparado.

### GREEN
- Helpers `normalizeCron` / `isValidCronClient` + error inline + bloqueo en `saveUpdateCheck`.

### REFACTOR / verificación
- `cd frontend && pnpm exec tsc -b` + `pnpm run build` + `pnpm run lint` + tests.

## Verificación de cierre

- [ ] Spec aprobado (`.approved`).
- [ ] RED → GREEN → REFACTOR.
- [ ] Frontend en verde.
- [ ] `docs/validate-cron-client-ui.md`.
- [ ] `openspec archive validate-cron-client-ui` + limpieza.
- [ ] PR a `develop` mergeado e issue #141 cerrado.

## Referencias

- #125; #135 / PR #136; #137 / PR #138. `docs/validate-cron-on-put.md`.
- Spec delta: `openspec/changes/validate-cron-client-ui/specs/updates/update-check/spec.md`.
