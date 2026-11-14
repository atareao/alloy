# Tasks

## 1. RED — Test que describe el invariante

- [x] 1.1 Crear `frontend/src/test-setup.test.tsx` con el caso encadenado A/B: el test A dispara el
      batch completado (`EventSourceMock.emit("update-progress", { done: true, ... })`) y afirma que
      `.ant-notification` está montado; el test B afirma que al empezar **no** queda ningún
      `.ant-notification` y que `document.body.childElementCount` volvió a su valor base.
- [x] 1.2 Añadir el test de mecanismo de `flushReactWork()`: un callback encolado con `setImmediate`
      debe haberse ejecutado al regresar el helper; una actualización de estado disparada fuera de
      `act` debe ser visible en el DOM al regresar el helper.
- [x] 1.3 Ejecutar `pnpm exec vitest run src/test-setup.test.tsx` y confirmar RED (1.1 falla por
      holder huérfano; 1.2 falla por helper inexistente).

## 2. GREEN — Implementación mínima

- [x] 2.1 En `frontend/src/test-setup.ts`: exportar `flushReactWork(rounds = 3)` (`await act(async () => {})`
      + N macrotareas con `setImmediate`).
- [x] 2.2 Desactivar la animación de los holders estáticos de antd con
      `ConfigProvider.config({ theme: { token: { motion: false } } })` (sin esto `destroy()` no
      desmonta en jsdom — medido).
- [x] 2.3 Añadir el `afterEach` global: `notification.destroy()` → `message.destroy()` →
      `cleanup()` → `flushReactWork()` (los dos holders estáticos de antd: `notification` en
      `App.tsx` y `message` en `ContainerRow.tsx`).
- [x] 2.4 Comprobar que el test de regresión pasa y que `pnpm run test` sigue verde (suite completa,
      sin `Unhandled Errors`), verificando que `motion: false` no rompe ningún test existente.

## 3. REFACTOR y verificación

- [x] 3.1 `pnpm exec tsc -b` y `pnpm run lint` limpios.
- [x] 3.2 Ejecutar el harness de repetición documentado (`--maxWorkers=1`, N repeticiones) y anotar
      el resultado como evidencia.
- [x] 3.3 Confirmar que ningún fichero de producción (`src/App.tsx`, hooks, componentes) ha cambiado.

## 4. Documentación y cierre

- [x] 4.1 `docs/frontend-test-isolation.md`: causa raíz, invariante, helper, harness de reproducción
      y limitaciones (por qué el race no se reproduce en local).
- [x] 4.2 Auditar con `@react-reviewer`, abrir PR a `develop`, archivar con `openspec archive`.
