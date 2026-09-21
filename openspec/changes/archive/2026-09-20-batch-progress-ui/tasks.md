# Tasks

## 1. Tests (RED)

- [ ] 1.1 Escribir test para BatchProgress que verifique que NO renderiza lista scrollable (no elementos con overflow:auto/maxHeight) y que el test falla inicialmente
- [ ] 1.2 Escribir test para BatchProgress que verifique que el contador muestra `{current} / {total} — {status}` con el primer contenedor no-done
- [ ] 1.3 Escribir test para BatchProgress que verifique que muestra "✅ Completado" cuando todos los contenedores están done
- [ ] 1.4 Escribir test para BatchProgress que verifique el resumen en vivo con contadores correctos (revisados, actualizados, errores, pendientes)
- [ ] 1.5 Verificar que los tests nuevos fallan (RED) y los tests existentes siguen pasando (GREEN)

## 2. Implementación (GREEN)

- [ ] 2.1 Eliminar lista scrollable, variables logEntries/logColor/logEmoji, y contadores redundantes de BatchProgress.tsx
- [ ] 2.2 Cambiar total para usar siempre batchProgress.total en vez de checkResults.updated
- [ ] 2.3 Añadir derivación de currentEntry/currentText desde progress Map
- [ ] 2.4 Añadir resumen en vivo con liveChecked, liveUpdated, liveErrors, livePending
- [ ] 2.5 Verificar que `npx tsc --noEmit` compila sin errores
- [ ] 2.6 Verificar que `npx vitest run` pasa todos los tests (nuevos y existentes)

## 3. Refactor

- [ ] 3.1 Ejecutar `npm run lint` y verificar que no hay warnings
- [ ] 3.2 Verificar que `npx vitest run` sigue pasando tras refactor