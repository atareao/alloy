# Tasks: Progress Bar Fix

## TDD Checklist

- [ ] RED: Escribir test que verifique que `batchProgress.current` se actualiza cuando `progress.checked` cambia
- [ ] GREEN: Actualizar `App.tsx` para sincronizar `batchProgress` con `progress` en el callback de state polling
- [ ] GREEN: Verificar que `BatchProgress.tsx` usa `batchProgress.current` correctamente
- [ ] REFACTOR: Ejecutar linter y formateo
- [ ] VERIFICAR: Tests existentes siguen pasando
- [ ] Archivar change proposal