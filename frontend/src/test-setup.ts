import "@testing-library/jest-dom";

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
