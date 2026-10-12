import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { EventSourceMock } from "./test-setup";
import { useRealtime } from "./useRealtime";
import type {
  StateResponse,
  ContainerEvent,
  UpdateProgress,
  NotifEvent,
} from "./types";

interface HarnessProps {
  onState: (s: StateResponse) => void;
  onContainerEvent?: (e: ContainerEvent) => void;
  onProgress?: (p: UpdateProgress) => void;
  onNotification?: (n: NotifEvent) => void;
  onError?: () => void;
}

function Harness(props: HarnessProps) {
  const { status } = useRealtime(props);
  return <div data-testid="status">{status}</div>;
}

const stateResponse: StateResponse = {
  containers: [],
  summary: {
    total: 1,
    running: 1,
    stopped: 0,
    paused: 0,
    with_updates: 0,
  },
  progress: {
    total: 0,
    checked: 0,
    updated: 0,
    errors: 0,
    checking: "",
  },
};

describe("useRealtime", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    EventSourceMock.reset();
  });

  it("opens a single SSE connection to /api/stream and reports status sse", () => {
    render(<Harness onState={vi.fn()} />);

    expect(EventSourceMock.instances).toHaveLength(1);
    const es = EventSourceMock.last()!;
    expect(es.url).toBe("/api/stream");
    expect(es.withCredentials).toBe(true);
    expect(screen.getByTestId("status").textContent).toBe("sse");
  });

  it("processes a snapshot event into onState", () => {
    const onState = vi.fn();
    render(<Harness onState={onState} />);

    act(() => {
      EventSourceMock.emit("snapshot", stateResponse);
    });

    expect(onState).toHaveBeenCalledTimes(1);
    expect(onState).toHaveBeenCalledWith(stateResponse);
  });

  it("processes a container-event event into onContainerEvent", () => {
    const onContainerEvent = vi.fn();
    render(<Harness onState={vi.fn()} onContainerEvent={onContainerEvent} />);

    const ev: ContainerEvent = {
      name: "nginx",
      action: "start",
      state: "running",
      timestamp: "2024-01-01T00:00:00Z",
    };

    act(() => {
      EventSourceMock.emit("container-event", ev);
    });

    expect(onContainerEvent).toHaveBeenCalledWith(ev);
  });

  it("processes an update-progress event into onProgress", () => {
    const onProgress = vi.fn();
    render(<Harness onState={vi.fn()} onProgress={onProgress} />);

    const progress: UpdateProgress = {
      container: "nginx",
      status: "checking",
      done: false,
      error: null,
      total: 3,
      checked: 1,
      updated: 0,
      errors: 0,
    };

    act(() => {
      EventSourceMock.emit("update-progress", progress);
    });

    expect(onProgress).toHaveBeenCalledWith(progress);
  });

  it("processes a notification event into onNotification", () => {
    const onNotification = vi.fn();
    render(
      <Harness onState={vi.fn()} onNotification={onNotification} />,
    );

    const notif: NotifEvent = {
      container: "nginx",
      status: "restarted",
      timestamp: "2024-01-01T00:00:00Z",
    };

    act(() => {
      EventSourceMock.emit("notification", notif);
    });

    expect(onNotification).toHaveBeenCalledWith(notif);
  });

  it("ignores malformed snapshot payloads without throwing", () => {
    const onState = vi.fn();
    render(<Harness onState={onState} />);

    act(() => {
      EventSourceMock.last()!.emitRaw("snapshot", "not-json");
    });

    expect(onState).not.toHaveBeenCalled();
  });

  it("falls back to polling after exceeding the SSE failure threshold", () => {
    const onState = vi.fn();
    const mockFetch = vi.fn().mockReturnValue(new Promise(() => {}));
    vi.stubGlobal("fetch", mockFetch);

    render(<Harness onState={onState} />);

    expect(screen.getByTestId("status").textContent).toBe("sse");

    const es = EventSourceMock.last()!;
    act(() => {
      es.emitError();
      es.emitError();
    });
    // Below threshold — still on SSE
    expect(screen.getByTestId("status").textContent).toBe("sse");

    act(() => {
      es.emitError();
    });

    expect(screen.getByTestId("status").textContent).toBe("polling");
    expect(mockFetch).toHaveBeenCalledWith(
      "/api/state",
      expect.objectContaining({ credentials: "include" }),
    );
  });

  it("closes the EventSource on unmount", () => {
    const { unmount } = render(<Harness onState={vi.fn()} />);
    const es = EventSourceMock.last()!;

    expect(es.closed).toBe(false);
    unmount();
    expect(es.closed).toBe(true);
  });
});
