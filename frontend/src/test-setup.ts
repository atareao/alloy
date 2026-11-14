import "@testing-library/jest-dom";
import { act, cleanup } from "@testing-library/react";
import { ConfigProvider, message, notification } from "antd";
import { afterEach } from "vitest";

/**
 * Globals del runtime de tests que `tsconfig.app.json` no declara.
 *
 * El proyecto de TS de la app sólo carga `types: ["vite/client"]`, así que no conoce
 * globals de Node como `setImmediate` (que vitest/jsdom sí exponen).
 *
 * Se declara aquí, en el único fichero de infraestructura de tests, para no tocar
 * `tsconfig.app.json` ni contaminar el type-check de la app cargando `@types/node`.
 * El callback se tipa como `(...args: never[]) => void` para aceptar cualquier firma
 * (p. ej. el `resolve` de una `Promise`, que lleva un parámetro `void`).
 */
declare global {
  function setImmediate(callback: (...args: never[]) => void): unknown;
}

// antd 6: `ConfigProvider.config` configura los *static methods* (`notification`,
// `message`), que montan su holder en `document.body` fuera del árbol de Testing
// Library. Sin desactivar la animación, `notification.destroy()` no desmonta en jsdom:
// el nodo se queda en `fade-leave` porque la animación de salida nunca termina (no hay
// `transitionend`), y el root huérfano deja trabajo de React agendado que revienta el
// teardown del entorno (`ReferenceError: window is not defined`).
ConfigProvider.config({ theme: { token: { motion: false } } });

// Polyfill ResizeObserver for jsdom (required by antd Card)
if (typeof ResizeObserver === "undefined") {
  class ResizeObserverMock {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  (globalThis as any).ResizeObserver = ResizeObserverMock;
}

/**
 * Mock EventSource for SSE connections in tests.
 *
 * Stores every listener so tests can emit named events, e.g.:
 *   EventSourceMock.emit("snapshot", stateResponse)
 *   EventSourceMock.last()?.emitError()
 *
 * Backwards compatible: instances created but never emitted keep behaving as
 * no-ops (listeners are registered but simply never fire).
 */
export class EventSourceMock {
  static instances: EventSourceMock[] = [];

  url: string;
  withCredentials: boolean;
  onerror: ((event: any) => void) | null = null;
  onopen: ((event: any) => void) | null = null;
  onmessage: ((event: any) => void) | null = null;
  readyState: number = 0;
  closed: boolean = false;
  CONNECTING: number = 0;
  OPEN: number = 1;
  CLOSED: number = 2;

  private listeners: Record<string, Array<(event: MessageEvent) => void>> = {};

  constructor(
    url: string,
    eventSourceInitDict?: { withCredentials?: boolean },
  ) {
    this.url = url;
    this.withCredentials = eventSourceInitDict?.withCredentials ?? false;
    EventSourceMock.instances.push(this);
  }

  addEventListener(name: string, cb: (event: MessageEvent) => void) {
    (this.listeners[name] ??= []).push(cb);
  }

  removeEventListener(name: string, cb: (event: MessageEvent) => void) {
    const list = this.listeners[name];
    if (!list) return;
    this.listeners[name] = list.filter((fn) => fn !== cb);
  }

  close() {
    this.closed = true;
    this.readyState = this.CLOSED;
  }

  dispatchEvent() {
    return false;
  }

  /** Test helper: emit a named event with JSON-serializable data. */
  emit(name: string, data: unknown): void {
    this.emitRaw(name, JSON.stringify(data));
  }

  /** Test helper: emit a named event with an already-serialized payload. */
  emitRaw(name: string, raw: string): void {
    const event = { type: name, data: raw } as MessageEvent;
    for (const cb of this.listeners[name] ?? []) cb(event);
  }

  /** Test helper: simulate a transport error. */
  emitError(): void {
    this.onerror?.(new Event("error"));
  }

  /** Test helper: simulate a successful (re)connection. */
  emitOpen(): void {
    this.onopen?.(new Event("open"));
  }

  static last(): EventSourceMock | undefined {
    return this.instances[this.instances.length - 1];
  }

  static emit(name: string, data: unknown): void {
    this.last()?.emit(name, data);
  }

  static reset(): void {
    this.instances = [];
  }
}

(globalThis as any).EventSource = EventSourceMock;

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

/**
 * Drena el trabajo pendiente del scheduler de React.
 *
 * React encola el *flush* de *passive effects* como **macrotarea**
 * (`MessageChannel`/`setImmediate`), no como microtarea, así que `await Promise.resolve()`
 * no basta. Se alternan rondas de macrotarea con `act()` para vaciar también la cola de
 * `act`, y se hacen varias rondas porque un flush puede agendar el siguiente.
 *
 * Por qué 3 rondas por defecto: React puede encadenar flushes (un flush agenda el
 * siguiente); 3 rondas cubren las cadenas observadas y el test del scheduler comprueba que
 * no queda trabajo pendiente.
 *
 * Fake timers: si un test usa `vi.useFakeTimers()`, `setImmediate` puede quedar simulado y
 * este drenado no haría nada; en ese caso hay que volver a `vi.useRealTimers()` antes del
 * teardown (o evitar fake timers con la API estática de antd).
 */
export async function flushReactWork(rounds = 3): Promise<void> {
  await act(async () => {});
  for (let i = 0; i < rounds; i++) {
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}

// Aislamiento entre tests: cierra los dos holders estáticos de antd (`notification` y
// `message`, este último usado por `frontend/src/components/ContainerRow.tsx`), que
// `cleanup()` no desmonta; desmonta los árboles de Testing Library y drena el scheduler,
// de modo que nada de React se ejecute después de que jsdom destruya el entorno.
afterEach(async () => {
  notification.destroy();
  message.destroy();
  cleanup();
  await flushReactWork();
});
