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

// Mock EventSource for SSE connections in tests
class EventSourceMock {
  url: string;
  withCredentials: boolean;
  onerror: ((event: any) => void) | null = null;
  readyState: number = 0;
  CONNECTING: number = 0;
  OPEN: number = 1;
  CLOSED: number = 2;

  constructor(url: string, eventSourceInitDict?: { withCredentials?: boolean }) {
    this.url = url;
    this.withCredentials = eventSourceInitDict?.withCredentials ?? false;
  }

  addEventListener() {}
  removeEventListener() {}
  close() {}
  dispatchEvent() { return false; }
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
