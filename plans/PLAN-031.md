# PLAN-031 — Documentar la trampa del hook SDD (commit → archive)

- **Issue:** #206
- **Change:** `openspec/changes/pre-commit-hook-sdd-trap/` (doc-only, `skip_specs: true`)
- **Rama:** `feature/pre-commit-hook-sdd-trap` (desde `develop`)
- **Fecha:** 2026-11-22

## Objetivo

Cerrar la deuda de documentación del hook `pre-commit`: la regla "código fuente exige proposal activo"
choca con archivar el change antes de commitear, y hoy el único síntoma es un commit rechazado sin
explicación del orden correcto.

## Alcance

- `docs/pre-commit-hook.md`: nueva sección `## Trampa: orden commit → archive`.
- Sin cambios en `.githooks/pre-commit`, `openspec/require-proposal.sh`, Rust ni TypeScript.

## Pasos

1. [x] Issue #206 y change proposal `pre-commit-hook-sdd-trap` (validado: 1 passed).
2. [x] RED: comprobar con `rg -F` que las cadenas clave no existen en el doc.
3. [ ] GREEN: escribir la sección con extensiones, definición de proposal activo, mensaje de error
       literal, causa, orden correcto y remedio; excluir explícitamente `--no-verify` como hábito.
4. [ ] Verificar que lo escrito coincide con `.githooks/pre-commit`.
5. [ ] `openspec validate --strict --specs` en verde (53 passed) y `openspec validate --changes`.
6. [ ] Commit gitmoji → archivar change → `docs/` + plan → PR a `develop` → cerrar issue #206 a mano.

## Verificación

```sh
rg -qF -- 'commit → archive' docs/pre-commit-hook.md
rg -qF -- 'COMMIT RECHAZADO' docs/pre-commit-hook.md
rg -qF -- 'git diff --cached' docs/pre-commit-hook.md
openspec validate --strict --specs
```

## Follow-ups

- Añadir el orden correcto al mensaje de error del hook (candidato a change aparte, toca `.githooks/`).