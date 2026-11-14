# Design

## Contexto y evidencia

Causa raíz verificada con un test temporal (borrado después) que medía el DOM entre dos tests
consecutivos del mismo fichero:

```
A base:   .ant-notification=0  bodyChildren=0
A final:  .ant-notification=1  bodyChildren=4
B inicio: .ant-notification=1   ← sigue montado al empezar el siguiente test
```

Cadena causal:

1. `src/App.tsx:193` usa `import { notification } from "antd"` (API **estática**). antd crea un
   **root de React propio** y lo injerta en `document.body`, fuera del contenedor de RTL.
2. El auto-`cleanup()` de Testing Library desmonta sólo el árbol renderizado por el test → el
   holder de antd queda huérfano y persiste entre tests.
3. Ese árbol mantiene su `setTimeout` de auto-cierre (`duration: 8`) y, sobre todo, React deja
   agendado un callback `NormalPriority` para vaciar *passive effects*.
4. `react-dom-client.development.js:17920` es literalmente
   `scheduleCallback$1(NormalPriority$1, function () { schedulerEvent = window.event; ... })`.
   Si jsdom ya destruyó el entorno (`window` eliminado del global), el callback lanza
   `ReferenceError: window is not defined`.
5. De ahí el carácter intermitente: normalmente el macrotask corre antes de que termine el
   teardown; con CPU saturada (CI, 1-2 vCPU, 9 ficheros en paralelo) pierde la carrera.

Reproducción local: **no** se consigue ni con `taskset -c 0` ni con `--maxWorkers=1` x5 (máquina
rápida). El defecto observable en local es el invariante roto del paso 2 (holder huérfano), que sí
es determinista y por eso se usa como test de regresión.

## Decisiones

### D1 — Desactivar la animación de antd en tests (imprescindible)

**Medido**: `notification.destroy()` por sí solo NO desmonta la notificación en jsdom. El nodo queda
en estado `ant-notification-fade-leave` indefinidamente, porque la animación de salida se completa
con eventos (`transitionend`/`animationend`) que jsdom nunca dispara:

```
A final:                 notif=1 notices=1 body=4   ← ya llamado destroy()
tick 0..10 (500 ms):     notices=1                  ← nunca desaparece
```

Con la animación desactivada para los holders estáticos de antd, `destroy()` sí desmonta:

```ts
ConfigProvider.config({ theme: { token: { motion: false } } });
```

```
A final:            notif=1 notices=1 body=4
tras destroy():     notif=0 notices=0 body=0
siguiente test:     notif=0 notices=0 body=0
```

`ConfigProvider.config` es la API oficial de antd para configurar los **static methods**
(`GlobalConfigProps` acepta `theme`), que es justo el caso de `notification` importado como función.
El mock `EventSourceMock` y `holderRender` no hacen falta: `theme.token.motion = false` es suficiente.

### D2 — `afterEach` global: cerrar el holder + drenar el scheduler

```ts
afterEach(async () => {
  notification.destroy();   // desmonta la notificación estática (ya sin animación)
  message.destroy();        // desmonta el holder estático de `message` (ContainerRow.tsx)
  cleanup();                // desmonta los árboles de RTL
  await flushReactWork();   // drena el trabajo pendiente del scheduler
});
```

Se hace en el `afterEach` global de `src/test-setup.ts` (afecta a toda la suite) y no en
`App.test.tsx`, porque el patrón "API estática + teardown" puede repetirse en cualquier fichero
futuro.

Drenar no es redundante: aunque la notificación se desmonte, React puede haber agendado ya el flush
de *passive effects* del commit. Tres rondas de macrotarea (no una) para tolerar cadenas de trabajo.
El drenado debe abarcar **macrotareas**, no sólo microtareas: el callback de React se encola con
`MessageChannel`/`setImmediate`, no con `Promise`, así que `await Promise.resolve()` no bastaría —
y el test de mecanismo lo distingue explícitamente.

Alternativas descartadas:

- **Migrar `src/App.tsx` a `App.useApp()`**: cambiaría código de producción (habría que envolver el
  árbol en `<AntApp>`), el holder estático es correcto en runtime (el `App` real nunca se desmonta)
  y el problema es exclusivamente de higiene de tests.
- **Silenciar `notification` con un spy solo en `App.test.tsx`**: parchearía ese fichero, no el
  invariante; cualquier test futuro que use la API estática volvería a romper el teardown.
- **Esperar a que la animación termine**: imposible, nunca termina en jsdom (medido arriba).

### D3 — `flushReactWork()` exportado y testeado

El helper vive en `test-setup.ts` con `export` para poder testearlo:

```ts
export async function flushReactWork(rounds = 3): Promise<void> {
  await act(async () => {});
  for (let i = 0; i < rounds; i++) {
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
```

`setImmediate` existe en el entorno de vitest (Node) aunque el DOM sea jsdom. Se alterna con
`await act(...)` para vaciar también la cola de `act` de React.

### D4 — Test de regresión determinista, no "reintentar el race"

El race no se puede reproducir en local, así que el test **no** intenta simularlo. Se testea el
invariante que lo hace imposible:

1. Test A: renderiza `App`, dispara el batch completado y comprueba que `.ant-notification`
   **está** visible (documenta que el flujo usa la API estática).
2. Test B (el siguiente): comprueba que `.ant-notification` es `null` y que los hijos de
   `document.body` volvieron al valor base → **falla hoy**, pasa tras el fix.
3. Tests de mecanismo de `flushReactWork()`: drena `setImmediate` pendiente y hace visible una
   actualización de estado disparada fuera de `act` (vía `EventSourceMock.emit`).

## Riesgos

- **Cobertura oculta**: el invariante se testea a nivel de DOM, no de "no errores no gestionados".
  El escenario "teardown sin errores" queda como verificación de CI (`pnpm run test` verde) y como
  comprobación repetida en el harness documentado.
- **Coste**: 3 macrotareas por test × 81 tests ≈ despreciable. `motion: false` acelera además los
  renders de antd en tests (sin animaciones), pero **hay que pasar la suite completa** para
  descartar que algún test dependiera de un estado intermedio de animación.
- **Cobertura del `sender`**: si otra librería monta holders estáticos, el mismo `afterEach` debe
  cubrirlo; se testea `notification` como caso representativo (el de `message` ya se cierra en el
  `afterEach`) y se deja documentado en `docs/frontend-test-isolation.md`.
