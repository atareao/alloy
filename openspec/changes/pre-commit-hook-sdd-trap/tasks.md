# Tasks

## 1. Sección de trampa en la documentación

- [ ] 1.1 Añadir a `docs/pre-commit-hook.md` la sección `## Trampa: orden commit → archive` con: extensiones que disparan el bloqueo (`*.rs`, `*.ts`, `*.tsx`, `*.js`, `*.jsx`), qué cuenta como proposal activo (`openspec/changes/*/proposal.md`), síntoma del fallo al archivar antes de commitear (mensaje literal `COMMIT RECHAZADO: Cambios en código fuente sin change proposal activo`), causa (el archive borra `openspec/changes/<feature>/`) y remedio correcto (commitear antes de archivar; si ya se archivó, abrir un change incremental nuevo).
- [ ] 1.2 Documentar que el hook evalúa solo lo **staged** (`git diff --cached`) y que `--no-verify` se desaconseja como práctica.
- [ ] 1.3 Revisar `## Follow-ups` y `## Decisiones` del documento para que no contradigan la sección nueva.

## 2. Verificación

- [ ] 2.1 Contrastar lo escrito con `.githooks/pre-commit`: extensiones, patrón de proposal activo y mensaje de error coinciden literalmente.
- [ ] 2.2 `openspec validate --strict --specs` sigue en verde (53 passed, 0 failed).
- [ ] 2.3 `openspec validate --change pre-commit-hook-sdd-trap` sin errores.