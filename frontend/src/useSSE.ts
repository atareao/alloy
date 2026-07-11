import { useEffect, useRef, useCallback } from "react";

export function useSSE(
  path: string,
  eventName: string,
  onMessage: (data: string) => void,
  onError?: () => void,
) {
  const esRef = useRef<EventSource | null>(null);
  const retriesRef = useRef(0);
  const maxRetries = 10;

  const cleanup = useCallback(() => {
    if (esRef.current) {
      esRef.current.close();
      esRef.current = null;
    }
  }, []);

  useEffect(() => {
    const connect = () => {
      cleanup();
      // Session cookie is sent automatically with credentials: "include" is implicit
      // but for EventSource we need the URL directly — cookies are sent automatically
      const es = new EventSource(path);
      esRef.current = es;

      es.addEventListener(eventName, (e: MessageEvent) => {
        retriesRef.current = 0;
        onMessage(e.data);
      });

      es.onerror = () => {
        es.close();
        onError?.();
        if (retriesRef.current < maxRetries) {
          retriesRef.current++;
          const delay = Math.min(1000 * Math.pow(2, retriesRef.current), 30000);
          setTimeout(connect, delay);
        }
      };
    };

    connect();
    return cleanup;
  }, [path, eventName, onMessage, onError, cleanup]);
}