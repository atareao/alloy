# Aislamiento de la suite de tests del frontend

Cómo se evita que el job `frontend` de CI falle de forma intermitente con
`ReferenceError: window is not defined` y qué invariante lo protege.

## Síntoma

El job `frontend` fallaba de vez en cuando (77 tests en verde, pero el proceso terminaba con
exit 1) con:

```
Vitest caught 1 unhandled error during the test run.
Uncaught Exception
ReferenceError: window is not defined
 ❯ node_modules/react-dom/cjs/react-dom-client.development.js:17920:15
 ❯ Immediate.performWorkUntilDeadline node_modules/scheduler/cjs/scheduler.development.js:45:48
 ❯ process.processImmediate node:internal/timers:574:21
This error originated in "src/App.test.tsx"
```

En local no se reproducía; en CI aparecía esporádicamente y desaparecía al relanzar el job.
Es un **race**, no un fallo funcional.

## Causa raíz (medida)

1. `frontend/src/App.tsx` usa la API **estática** de antd (`import { notification } from "antd"`
   → `notification.success(...)`). Esa API monta un **root de React propio** en `document.body`,
   **fuera** de la raíz que renderiza Testing Library.
2. `cleanup()` de Testing Library desmonta sólo el árbol del test; el holder de antd **sobrevive**
   al test (medido entre dos tests consecutivos: `.ant-notification` = 1 al empezar el siguiente).
3. Además, la animación de salida de antd (clase `ant-notification-fade-leave`) se completa con
   eventos `transitionend`/`animationend` que **jsdom nunca dispara**. Por eso `notification.destroy()`
   por sí solo **no** desmonta la notificación: se queda en `fade-leave` indefinidamente.
4. Ese árbol huérfano mantiene su root de React vivo y deja agendado un *flush* de
   *passive effects* como **macrotarea**.
5. `react-dom-client.development.js` ejecuta, en ese callback:

   ```js
   scheduleCallback$1(NormalPriority$1, function () {
     schedulerEvent = window.event;   // ← si jsdom ya destruyó el entorno, ReferenceError
     flushPassiveEffects();
   ```

   Si el teardown del entorno gana la carrera al macrotask pendiente, `window` ya no existe.
   Con CPU saturada (CI: 1-2 vCPU, 8 ficheros en paralelo) pierde la carrera; en una máquina
   rápida casi nunca.
6. Sólo afectaba a `App.test.tsx` porque es el único test que dispara el flujo de "batch
   completado", que es lo que llama a la API estática.

## El fix

Todo vive en `frontend/src/test-setup.ts`, el fichero de infraestructura de tests. **No se toca
código de producción** (la API estática es correcta en runtime: el `App` real nunca se desmonta).

### 1. Desactivar la animación de antd en tests

```ts
import { ConfigProvider } from "antd";
ConfigProvider.config({ theme: { token: { motion: false } } });
```

`ConfigProvider.config` es la API oficial de antd para configurar los **static methods**
(`notification`, `message`), que es justo el caso de `notification` importado como función.
Sin `motion: false`, `notification.destroy()` no desmonta en jsdom (punto 3 de la causa raíz).
Con `motion: false`, `destroy()` deja `notif=0 notices=0 body=0`.

### 2. `afterEach` global: cerrar + desmontar + drenar

```ts
import { afterEach } from "vitest";
import { act, cleanup } from "@testing-library/react";
import { message, notification } from "antd";

afterEach(async () => {
  notification.destroy();   // desmonta el holder estático de antd (ya sin animación)
  message.destroy();        // ídem para el holder de `message` (usado por ContainerRow.tsx)
  cleanup();                // desmonta los árboles de Testing Library
  await flushReactWork();   // drena el trabajo pendiente del scheduler de React
});
```

El orden importa. Es `afterEach` global (no por fichero) porque el patrón "API estática +
teardown" puede repetirse en cualquier test futuro. Además del holder de `notification`
(`App.tsx`), cierra el de `message`, que usa `frontend/src/components/ContainerRow.tsx`
(`message.success/error/info`): ambos montan su root de React en `document.body` y ninguno lo
desmonta `cleanup()`.

### 3. `flushReactWork()`: drenar macrotareas, no sólo microtareas

```ts
export async function flushReactWork(rounds = 3): Promise<void> {
  await act(async () => {});
  for (let i = 0; i < rounds; i++) {
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
```

React encola su *flush* de *passive effects* con **`setImmediate`** (así lo muestra el propio
stack trace de CI: `Immediate.performWorkUntilDeadline` → `process.processImmediate`), no con una
`Promise`. Un helper que sólo hiciera `await Promise.resolve()` **no** serviría: el callback
seguiría pendiente al destruirse el entorno. Se alternan rondas de `act()` y `setImmediate`, y se
hacen varias porque un *flush* puede agendar el siguiente.

> Nota de tipos: `tsconfig.app.json` carga sólo `types: ["vite/client"]`, así que `setImmediate`
> no está declarado. Se declara en un `declare global` **dentro del propio fichero de tests** para
> no cargar `@types/node` en el type-check de la app. `afterEach` sí se importa de `vitest`.

> Nota de fake timers: el drenado se apoya en `setImmediate`. Si un test activa
> `vi.useFakeTimers()`, `setImmediate` puede quedar simulado y `flushReactWork()` no drenaría nada;
> hay que volver a `vi.useRealTimers()` antes del teardown (o evitar fake timers con la API
> estática de antd).

## El invariante (tests de regresión)

`frontend/src/test-setup.test.tsx` describe el contrato de forma **determinista**: no intenta
reproducir el race (imposible en local), sino el estado que lo hace imposible.

- **Precondición**: un test dispara la API estática (`EventSourceMock.emit("update-progress",
  { done: true, ... })`) y comprueba que `.ant-notification-notice` **está** montado.
- **Invariante**: el test siguiente comprueba que al empezar **no** queda ningún
  `.ant-notification`, ningún `.ant-notification-notice` y `document.body.childElementCount === 0`.
  Antes del fix este test **fallaba** (holder huérfano); ahora pasa.
- **Mecanismo**: `flushReactWork()` drena un callback encolado con `setImmediate`, y deja el
  scheduler de React sin trabajo (ningún macrotask adicional muta el DOM).

## Cómo reproducir / verificar

```bash
cd frontend
pnpm run test                                             # suite completa (9 ficheros / 81 tests)
pnpm exec vitest run src/test-setup.test.tsx               # invariante del aislamiento
# Harness de repetición (el race sólo aparece con CPU saturada):
for i in $(seq 1 8); do
  pnpm exec vitest run src/App.test.tsx src/test-setup.test.tsx --maxWorkers=1
done
```

## Limitaciones

- El race original **no se reproduce** de forma fiable en una máquina rápida ni con
  `--maxWorkers=1`; por eso el test cubre el invariante (holder huérfano), no la carrera.
- `motion: false` (vía `ConfigProvider.config`) afecta sólo a los **holders estáticos** que antd
  configura globalmente: `notification`, `message` y los `Modal.*` estáticos. **No** desactiva la
  animación de los componentes renderizados dentro del árbol del test (Modal/Drawer/Tooltip siguen
  animándose), así que ningún test que dependa de una animación de componente cambia de
  comportamiento por esto. Acelera además los renders de los avisos estáticos.
- El `afterEach` también destruye el holder de `message` (usado por
  `frontend/src/components/ContainerRow.tsx`), no sólo el de `notification`.
- Si otra librería monta holders estáticos, el mismo `afterEach` debe cubrirlo; aquí se testea
  `notification` como caso representativo.
