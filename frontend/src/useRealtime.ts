import { useCallback, useEffect, useRef, useState } from "react";
import { useStatePoll } from "./useStatePoll";
import type {
  ContainerEvent,
  NotifEvent,
  StateResponse,
  UpdateProgress,
} from "./types";

export type RealtimeStatus = "sse" | "polling" | "down";

export interface UseRealtimeOptions {
  onState: (state: StateResponse) => void;
  onContainerEvent?: (event: ContainerEvent) => void;
  onProgress?: (progress: UpdateProgress) => void;
  onNotification?: (notif: NotifEvent) => void;
  onError?: () => void;
}

/** Consecutive SSE errors tolerated before switching to the long-poll fallback. */
const MAX_SSE_FAILURES = 3;
/** How often we retry SSE while running on the polling fallback. */
const SSE_RETRY_MS = 30_000;

/**
 * Primary realtime transport for the UI.
 *
 * Opens a single `EventSource("/api/stream")` and dispatches the four named
 * events (`snapshot`, `container-event`, `update-progress`, `notification`).
 * If `EventSource` is unavailable or the connection fails repeatedly, it
 * falls back to the long-poll `useStatePoll` hook and periodically retries
 * SSE. Callbacks are kept in refs so the listeners never go stale.
 */
export function useRealtime(options: UseRealtimeOptions): {
  status: RealtimeStatus;
} {
  const [status, setStatus] = useState<RealtimeStatus>(() =>
    typeof EventSource === "undefined" ? "polling" : "sse",
  );

  const statusRef = useRef(status);
  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  // Keep the latest callbacks without re-subscribing the stream.
  const onStateRef = useRef(options.onState);
  const onContainerEventRef = useRef(options.onContainerEvent);
  const onProgressRef = useRef(options.onProgress);
  const onNotificationRef = useRef(options.onNotification);
  const onErrorRef = useRef(options.onError);

  useEffect(() => {
    onStateRef.current = options.onState;
  }, [options.onState]);
  useEffect(() => {
    onContainerEventRef.current = options.onContainerEvent;
  }, [options.onContainerEvent]);
  useEffect(() => {
    onProgressRef.current = options.onProgress;
  }, [options.onProgress]);
  useEffect(() => {
    onNotificationRef.current = options.onNotification;
  }, [options.onNotification]);
  useEffect(() => {
    onErrorRef.current = options.onError;
  }, [options.onError]);

  const failuresRef = useRef(0);

  // ── SSE transport (active only while status === "sse") ────────────
  useEffect(() => {
    if (status !== "sse" || typeof EventSource === "undefined") return;

    const es = new EventSource("/api/stream", { withCredentials: true });

    const onSnapshot = (e: Event) => {
      failuresRef.current = 0;
      try {
        const data = JSON.parse((e as MessageEvent).data) as StateResponse;
        onStateRef.current(data);
      } catch {
        // Ignore malformed payloads — the next snapshot will resync.
      }
    };

    const onContainerEvent = (e: Event) => {
      try {
        const data = JSON.parse((e as MessageEvent).data) as ContainerEvent;
        onContainerEventRef.current?.(data);
      } catch {
        /* ignore malformed */
      }
    };

    const onUpdateProgress = (e: Event) => {
      try {
        const data = JSON.parse((e as MessageEvent).data) as UpdateProgress;
        onProgressRef.current?.(data);
      } catch {
        /* ignore malformed */
      }
    };

    const onNotification = (e: Event) => {
      try {
        const data = JSON.parse((e as MessageEvent).data) as NotifEvent;
        onNotificationRef.current?.(data);
      } catch {
        /* ignore malformed */
      }
    };

    es.addEventListener("snapshot", onSnapshot);
    es.addEventListener("container-event", onContainerEvent);
    es.addEventListener("update-progress", onUpdateProgress);
    es.addEventListener("notification", onNotification);

    const onOpen = () => {
      failuresRef.current = 0;
    };

    const onError = () => {
      failuresRef.current += 1;
      if (failuresRef.current >= MAX_SSE_FAILURES) {
        es.close();
        setStatus("polling");
      }
    };

    es.onopen = onOpen;
    es.onerror = onError;

    return () => {
      es.removeEventListener("snapshot", onSnapshot);
      es.removeEventListener("container-event", onContainerEvent);
      es.removeEventListener("update-progress", onUpdateProgress);
      es.removeEventListener("notification", onNotification);
      es.onopen = null;
      es.onerror = null;
      es.close();
    };
  }, [status]);

  // ── While on the fallback, periodically retry SSE ─────────────────
  useEffect(() => {
    if (status === "sse" || typeof EventSource === "undefined") return;
    const timer = setTimeout(() => {
      failuresRef.current = 0;
      setStatus("sse");
    }, SSE_RETRY_MS);
    return () => clearTimeout(timer);
  }, [status]);

  // ── Polling fallback (reuses useStatePoll, gated by status) ───────
  useStatePoll(
    useCallback((state: StateResponse) => {
      onStateRef.current(state);
    }, []),
    useCallback(() => {
      setStatus("down");
      onErrorRef.current?.();
    }, []),
    { enabled: status === "polling" },
  );

  return { status };
}
