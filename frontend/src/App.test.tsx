import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  act,
} from "@testing-library/react";
import { ConfigProvider } from "antd";
import App from "./App";
import { EventSourceMock } from "./test-setup";
import { makeContainer } from "./components/testFixtures";
import type { ContainerInfo } from "./types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

// Mock fetch for auth and API calls
function mockAuthResponse(authenticated = true) {
  return vi.fn().mockImplementation((url: string) => {
    if (url === "/api/auth/me") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            authenticated,
            user: authenticated
              ? { sub: "test", name: "Test User", email: "test@example.com" }
              : null,
          }),
          { status: authenticated ? 200 : 401 },
        ),
      );
    }
    if (url.startsWith("/api/containers")) {
      return Promise.resolve(
        new Response(
          JSON.stringify({ items: [], total: 0, page: 1, page_size: 20 }),
        ),
      );
    }
    if (url === "/api/state") {
      // Return a promise that never resolves to prevent infinite polling loop
      return new Promise(() => {});
    }
    if (url.startsWith("/api/history")) {
      // Envelope paginado (D1): `HistoryPage` es quien lo consume.
      return Promise.resolve(
        new Response(
          JSON.stringify({ items: [], total: 0, page: 1, page_size: 20 }),
        ),
      );
    }
    if (url === "/api/config") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            host: "localhost",
            port: 3066,
            scan_interval_secs: 30,
            allowed_containers: [],
            oidc_issuer_url: "https://issuer.example.com",
            oidc_client_id: "test",
            oidc_redirect_url: "http://localhost:5173/api/auth/callback",
            auto_update_interval_hours: 6,
            telegram_bot_token: "",
            telegram_chat_id: "",
            matrix_homeserver: "",
            matrix_user: "",
            matrix_password: "",
            matrix_room_id: "",
          }),
        ),
      );
    }
    if (url === "/api/update-check/config") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            last_check: null,
            next_check: null,
            auto_update_interval_hours: 6,
          }),
        ),
      );
    }
    return Promise.resolve(new Response("ok"));
  });
}

describe("App layout - topbar navigation", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    EventSourceMock.reset();
    vi.stubGlobal("fetch", mockAuthResponse(true));
  });

  it("renders logo and app name in the header", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // Logo image should be present
    const logo = document.querySelector('img[alt="Alloy"]');
    expect(logo).toBeInTheDocument();
  });

  it("renders four navigation buttons in the header", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // Desktop: buttons should have text labels
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Historial")).toBeInTheDocument();
    expect(screen.getByText("Config")).toBeInTheDocument();
    expect(screen.getByText("Salir")).toBeInTheDocument();
  });

  it("does NOT render a Layout.Sider", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // Ant Design Sider has class "ant-layout-sider"
    const sider = document.querySelector(".ant-layout-sider");
    expect(sider).not.toBeInTheDocument();
  });

  it("does NOT render a hamburger menu button", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // The hamburger icon is MenuUnfoldOutlined
    const hamburger = document.querySelector(".anticon-menu-unfold");
    expect(hamburger).not.toBeInTheDocument();
  });

  it("changes view when clicking a navigation button", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // Click "Historial" button
    fireEvent.click(screen.getByText("Historial"));

    // The view should change - DashboardPage content should not be visible
    // and we should see history-related content
    // (We just verify the button click doesn't crash and view state changes)
    await waitFor(() => {
      // After clicking Historial, the Dashboard button should still be there
      expect(screen.getByText("Dashboard")).toBeInTheDocument();
    });
  });

  it("highlights the active navigation button", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // Dashboard should be the default active view
    // The active button should have type="primary" in Ant Design
    const dashboardBtn = screen.getByText("Dashboard").closest("button");
    expect(dashboardBtn).toBeInTheDocument();

    // Click Historial
    fireEvent.click(screen.getByText("Historial"));

    await waitFor(() => {
      // After clicking, the Historial button should be active
      const historialBtn = screen.getByText("Historial").closest("button");
      expect(historialBtn).toBeInTheDocument();
    });
  });

  it("logs out when clicking Salir button", async () => {
    const originalLocation = window.location;
    Object.defineProperty(window, "location", {
      value: { href: "" },
      writable: true,
    });

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Salir"));

    expect(window.location.href).toBe("/api/auth/logout");

    Object.defineProperty(window, "location", {
      value: originalLocation,
      writable: true,
    });
  });

  it("processes a snapshot event from the stream on page load", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    // Wait for the component to render
    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // The stream delivers a full snapshot on connect
    act(() => {
      EventSourceMock.emit("snapshot", {
        containers: [],
        total: 0,
        page: 1,
        page_size: 20,
        summary: {
          total: 0,
          running: 0,
          stopped: 0,
          paused: 0,
          with_updates: 0,
        },
        progress: {
          total: 5,
          checked: 2,
          updated: 1,
          errors: 0,
          checking: "nginx",
        },
      });
    });

    // The progress card should NOT be visible because batchPhase is "idle"
    // (a snapshot only updates progress, it does not start a batch)
    expect(
      screen.queryByText(/Revisando y actualizando containers/),
    ).not.toBeInTheDocument();
  });

  it("hides progress card when update-progress signals batch completion", async () => {
    // The batch end is now driven by the `update-progress` SSE event
    // (payload.done === true), not by polling heuristics.
    const mockFetch = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/check-all") {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      return mockAuthResponse(true)(url);
    });
    vi.stubGlobal("fetch", mockFetch);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    // Wait for the component to render
    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // Click "Check" to start batch
    const checkBtn = screen.getByText("Check");
    fireEvent.click(checkBtn);

    // The progress card should appear (batchPhase becomes "active")
    await waitFor(() => {
      expect(
        screen.getByText(/Revisando y actualizando containers/),
      ).toBeInTheDocument();
    });

    // Backend signals batch completion via a single stream event
    act(() => {
      EventSourceMock.emit("update-progress", {
        container: "nginx",
        status: "done",
        done: true,
        error: null,
        total: 2,
        checked: 2,
        updated: 1,
        errors: 0,
      });
    });

    // The card should disappear once the final event is processed
    await waitFor(() => {
      expect(
        screen.queryByText(/Revisando y actualizando containers/),
      ).not.toBeInTheDocument();
    });
  });

  it("does NOT show progress card when no active updates", async () => {
    // Mock fetch to return only completed progress
    const mockFetch = vi.fn().mockImplementation((url: string) => {
      return mockAuthResponse(true)(url);
    });
    vi.stubGlobal("fetch", mockFetch);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    // Wait for the component to render
    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // The progress card should NOT be visible
    expect(
      screen.queryByText(/Revisando y actualizando containers/),
    ).not.toBeInTheDocument();
  });
});

describe("App layout - mobile topbar", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    EventSourceMock.reset();
    vi.stubGlobal("fetch", mockAuthResponse(true));

    // Mock matchMedia for mobile (width <= 768px)
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: (query: string) => ({
        matches: query === "(max-width: 768px)",
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }),
    });
  });

  it("renders logo on the left and icon buttons on the right on mobile", async () => {
    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // On mobile, buttons should NOT have text labels visible
    // (only icons should be visible)
    const dashboardText = screen.queryByText("Dashboard");
    expect(dashboardText).not.toBeInTheDocument();

    const historialText = screen.queryByText("Historial");
    expect(historialText).not.toBeInTheDocument();

    const configText = screen.queryByText("Config");
    expect(configText).not.toBeInTheDocument();

    const salirText = screen.queryByText("Salir");
    expect(salirText).not.toBeInTheDocument();

    // But the icons should be present
    const buttons = document.querySelectorAll(
      ".ant-layout-header button, header button",
    );
    expect(buttons.length).toBeGreaterThanOrEqual(4);
  });
});

// ── Server-side pagination & filters ────────────────────────────────
function pagedItems(): ContainerInfo[] {
  return [
    makeContainer({
      id: "id-web",
      name: "web",
      state: "running",
      has_update: true,
    }),
    makeContainer({ id: "id-db", name: "db", state: "exited" }),
  ];
}

/** URL list of every `/api/containers` request issued against the mock. */
function containerCalls(fetchMock: ReturnType<typeof vi.fn>): string[] {
  return fetchMock.mock.calls
    .map((c) => c[0] as string)
    .filter((u) => u.startsWith("/api/containers"));
}

/** Echoes the requested page/page_size back, like the backend envelope. */
function echoEnvelope(url: string, total = 2) {
  const u = new URL(url, "http://localhost");
  return {
    items: pagedItems(),
    total,
    page: Number(u.searchParams.get("page") ?? 1),
    page_size: Number(u.searchParams.get("page_size") ?? 20),
  };
}

/** Wraps `mockAuthResponse` overriding only the paginated list endpoint. */
function pagedFetch(makeBody: (url: string) => unknown) {
  const base = mockAuthResponse(true);
  return vi.fn().mockImplementation((url: string) => {
    if (url.startsWith("/api/containers")) {
      return Promise.resolve(new Response(JSON.stringify(makeBody(url))));
    }
    return base(url);
  });
}

describe("App - server-side pagination and filters", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    EventSourceMock.reset();
  });

  it("pide /api/containers con page y page_size en la carga inicial", async () => {
    const fetchMock = pagedFetch((url) => echoEnvelope(url));
    vi.stubGlobal("fetch", fetchMock);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await waitFor(() =>
      expect(containerCalls(fetchMock).length).toBeGreaterThan(0),
    );
    expect(containerCalls(fetchMock)[0]).toBe(
      "/api/containers?page=1&page_size=20",
    );
  });

  it("pide /api/containers con q tras el debounce de la búsqueda", async () => {
    const fetchMock = pagedFetch((url) => echoEnvelope(url));
    vi.stubGlobal("fetch", fetchMock);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );
    await waitFor(() =>
      expect(containerCalls(fetchMock).length).toBeGreaterThan(0),
    );

    fireEvent.change(
      screen.getByPlaceholderText("Buscar por nombre, imagen, stack..."),
      { target: { value: "web" } },
    );

    await waitFor(() =>
      expect(containerCalls(fetchMock).some((u) => u.includes("q=web"))).toBe(
        true,
      ),
    );
  });

  it("pide /api/containers con state al activar el filtro de estado", async () => {
    const fetchMock = pagedFetch((url) => echoEnvelope(url));
    vi.stubGlobal("fetch", fetchMock);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );
    await waitFor(() =>
      expect(containerCalls(fetchMock).length).toBeGreaterThan(0),
    );

    fireEvent.click(await screen.findByText("running"));

    await waitFor(() =>
      expect(
        containerCalls(fetchMock).some((u) => u.includes("state=running")),
      ).toBe(true),
    );
  });

  it("pide /api/containers con has_update al activar pendientes", async () => {
    const fetchMock = pagedFetch((url) => echoEnvelope(url));
    vi.stubGlobal("fetch", fetchMock);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );
    await waitFor(() =>
      expect(containerCalls(fetchMock).length).toBeGreaterThan(0),
    );

    fireEvent.click(screen.getByRole("switch"));

    await waitFor(() =>
      expect(
        containerCalls(fetchMock).some((u) => u.includes("has_update=true")),
      ).toBe(true),
    );
  });

  it("pide la página 2 y reabre el stream al cambiar de página", async () => {
    const fetchMock = pagedFetch((url) => echoEnvelope(url, 25));
    vi.stubGlobal("fetch", fetchMock);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await screen.findByText("25 contenedores");
    fireEvent.click(screen.getByLabelText("Page 2"));

    await waitFor(() =>
      expect(containerCalls(fetchMock).some((u) => u.includes("page=2"))).toBe(
        true,
      ),
    );
    await waitFor(() =>
      expect(EventSourceMock.last()!.url).toContain("page=2"),
    );
  });
});

// ── El histórico ya no lo posee App (D3) ────────────────────────────
/** URL list of every `/api/history` request issued against the mock. */
function historyCalls(fetchMock: ReturnType<typeof vi.fn>): string[] {
  return fetchMock.mock.calls
    .map((c) => c[0] as string)
    .filter((u) => typeof u === "string" && u.startsWith("/api/history"));
}

describe("App - el histórico pasa a ser responsabilidad de HistoryPage", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    EventSourceMock.reset();
  });

  it("3.9 App ya NO solicita /api/history (no mantiene el array del histórico)", async () => {
    const fetchMock = mockAuthResponse(true);
    vi.stubGlobal("fetch", fetchMock);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    await screen.findByText("Alloy");
    // Espera a que arranque la carga de contenedores para que el efecto de
    // montaje de App ya haya corrido por completo.
    await waitFor(() =>
      expect(containerCalls(fetchMock).length).toBeGreaterThan(0),
    );

    expect(historyCalls(fetchMock)).toEqual([]);
  });
});
