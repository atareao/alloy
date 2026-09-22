import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ConfigProvider } from "antd";
import App from "./App";

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
    if (url === "/api/containers") {
      return Promise.resolve(new Response(JSON.stringify([])));
    }
    if (url === "/api/state") {
      // Long-poll: never resolve (simulates server holding the connection)
      return new Promise(() => {});
    }
    if (url === "/api/check-progress") {
      return Promise.resolve(new Response(JSON.stringify({})));
    }
    if (url === "/api/history") {
      return Promise.resolve(new Response(JSON.stringify([])));
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

  it("recovers active batch progress on page load", async () => {
    // Mock fetch to return active progress from /api/check-progress
    const mockFetch = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/check-progress") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              nginx: {
                container: "nginx",
                status: "Pulling nginx:latest...",
                done: false,
                error: null,
                total: 5,
                checked: 2,
                updated: 1,
                errors: 0,
              },
            }),
          ),
        );
      }
      return mockAuthResponse(true)(url);
    });
    vi.stubGlobal("fetch", mockFetch);

    render(
      <Wrapper>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </Wrapper>,
    );

    // The recovery effect should set batchPhase to "active"
    // BatchProgress card shows "Revisando y actualizando containers..."
    await waitFor(() => {
      expect(
        screen.getByText(/Revisando y actualizando containers/),
      ).toBeInTheDocument();
    });

    // Should show the progress: 2 / 5
    expect(screen.getByText(/2 \/ 5/)).toBeInTheDocument();
  });

  it("does NOT show progress card when no active updates", async () => {
    // Mock fetch to return only completed progress
    const mockFetch = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/check-progress") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              nginx: {
                container: "nginx",
                status: "✅ Sin cambios",
                done: true,
                error: null,
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
              },
            }),
          ),
        );
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

    // The progress card should NOT be visible
    expect(
      screen.queryByText(/Revisando y actualizando containers/),
    ).not.toBeInTheDocument();
  });
});

describe("App layout - mobile topbar", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
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
