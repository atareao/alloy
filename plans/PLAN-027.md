# PLAN-027 — Política de cero warnings de oxlint

- Issue: #189 (follow-up de #179)
- Tipo: verificación + documentación (sin código de aplicación)
- Change OpenSpec: no aplica (no hay cambio de código)
- Doc final: `docs/oxlint-zero-warnings.md`

## Objetivo

Dejar constancia verificable de que la baseline de lint está limpia con
`oxlint --deny-warnings` y de que CI la hace cumplir, y cerrar el follow-up.

## Alcance

1. Ejecutar `pnpm run lint` en `frontend/` → 0 warnings, exit 0.
2. Confirmar que `ci.yml` ejecuta `pnpm run lint` en cada PR.
3. Documentar en `docs/oxlint-zero-warnings.md`: comando, política (`--deny-warnings`,
   cualquier warning rompe CI), dónde se configura (`.oxlintrc.json`,
   `package.json`) y cómo añadir una excepción justificada.
4. Actualizar `HANDOFF.md` (follow-up cerrado) y cerrar #189.

## Tareas

- [ ] `cd frontend && pnpm run lint` → sin warnings.
- [ ] Verificar el paso de lint en `.github/workflows/ci.yml`.
- [ ] Escribir `docs/oxlint-zero-warnings.md`.
- [ ] Actualizar `HANDOFF.md` y cerrar #189.