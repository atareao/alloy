import { useEffect, useRef } from "react";
import type { StateResponse } from "./types";

export function useStatePoll(
  onState: (state: StateResponse) => void,
  onError?: () => void,
) {
  const abortRef = useRef<AbortController | null>(null);
  const retriesRef = useRef(0);
  const maxRetries = 10;
  const onStateRef = useRef(onState);
  const onErrorRef = useRef(onError);

  useEffect(() => {
    onStateRef.current = onState;
  }, [onState]);

  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const poll = async () => {
      const controller = new AbortController();
      abortRef.current = controller;

      try {
        const res = await fetch("/api/state", {
          credentials: "include",
          signal: controller.signal,
        });
        if (!res.ok) throw new Error("Non-200 response");
        const data: StateResponse = await res.json();
        if (cancelled) return;
        retriesRef.current = 0;
        onStateRef.current(data);
        // Immediately poll again (long-poll: server holds the request until data is available)
        if (!cancelled) poll();
      } catch (err: any) {
        if (err.name === "AbortError") return;
        if (cancelled) return;
        if (retriesRef.current < maxRetries) {
          const delay = Math.min(1000 * Math.pow(2, retriesRef.current), 30000);
          retriesRef.current++;
          timer = setTimeout(poll, delay);
        } else {
          onErrorRef.current?.();
        }
      }
    };

    poll();

    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
      if (abortRef.current) abortRef.current.abort();
    };
  }, []);
}
