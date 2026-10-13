import { describe, it, expect, vi, beforeEach, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { ConfigProvider } from "antd";
import ConfigPage from "./ConfigPage";
import type { AppConfig } from "../types";

// Polyfill ResizeObserver for jsdom (required by antd Card)
beforeAll(() => {
  if (typeof ResizeObserver === "undefined") {
    class ResizeObserverMock {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
    (globalThis as any).ResizeObserver = ResizeObserverMock;
  }
});

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

function makeConfig(overrides: Partial<AppConfig> = {}): AppConfig {
  return {
    oidc_configured: true,
    port: 3066,
    timezone: "UTC",
    telegram_configured: false,
    telegram_token: null,
    telegram_chat_id: null,
    matrix_configured: false,
    matrix_token: null,
    matrix_homeserver: null,
    matrix_room: null,
    webhook_configured: false,
    valet_configured: false,
    valet_url: null,
    valet_token: null,
    version: "0.0.0",
    build_date: "2024-01-01T00:00:00Z",
    repo_url: "https://example.com/repo",
    ...overrides,
  };
}

interface FetchCall {
  url: string;
  method: string;
  body: unknown;
}

/**
 * Installs a fetch mock that answers ConfigPage's mount-time requests
 * (`/api/update-check/config`, `/api/update-policies/default`) and records
 * every call so tests can assert on `PUT /api/config` and
 * `POST /api/notifications/test` payloads.
 */
function installFetchMock(config: AppConfig): FetchCall[] {
  const calls: FetchCall[] = [];
  const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
    const method = opts?.method ?? "GET";
    const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
    calls.push({ url, method, body });

    if (url === "/api/update-check/config") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            cron: "0 */6 * * *",
            enabled: false,
            notify: false,
            last_run_at: null,
            next_run_at: null,
          }),
        ),
      );
    }
    if (url === "/api/update-policies/default") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            action: "pull-restart",
            cleanup_old_image: false,
            rollback_on_failure: false,
          }),
        ),
      );
    }
    if (url === "/api/config" && method === "PUT") {
      // Echo back the merged config, like the real backend would.
      return Promise.resolve(new Response(JSON.stringify(config)));
    }
    if (url === "/api/notifications/test") {
      return Promise.resolve(
        new Response(JSON.stringify({ status: "ok", channel: "valet" })),
      );
    }
    return Promise.resolve(new Response("ok"));
  });
  vi.stubGlobal("fetch", mock);
  return calls;
}

function renderConfigPage(config: AppConfig) {
  const setConfig = vi.fn();
  const calls = installFetchMock(config);
  render(
    <Wrapper>
      <ConfigPage
        config={config}
        setConfig={setConfig}
        colorScheme="dark"
        setColorScheme={() => {}}
      />
    </Wrapper>,
  );
  return { calls, setConfig };
}

function getValetCard(): HTMLElement {
  const title = screen.getByText("🪐 Valet");
  const card = title.closest(".ant-card");
  if (!card) throw new Error("Valet card not found");
  return card as HTMLElement;
}

describe("ConfigPage — tarjeta Valet", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("renderiza la tarjeta Valet con la URL y el token configurados", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
      valet_token: "val_secret_token",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("🪐 Valet")).toBeInTheDocument();
    });

    // URL visible in the (plain) input
    expect(screen.getByDisplayValue("https://valet.test")).toBeInTheDocument();
    // Token visible (non-empty) in the password input
    const tokenInput = screen.getByDisplayValue("val_secret_token");
    expect(tokenInput).toBeInTheDocument();
  });

  it("expone nombre accesible en el switch y los campos (a11y)", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
      valet_token: "val_abc",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("🪐 Valet")).toBeInTheDocument();
    });

    const card = getValetCard();

    expect(
      within(card).getByRole("switch", { name: "Activar Valet" }),
    ).toBeInTheDocument();
    expect(
      within(card).getByLabelText("URL de Valet"),
    ).toBeInTheDocument();
    expect(
      within(card).getByLabelText("Ingest key de Valet"),
    ).toBeInTheDocument();
  });

  it("guarda la URL y el token no vacíos al activar el switch", async () => {
    const config = makeConfig({
      valet_configured: false,
      valet_url: null,
      valet_token: null,
    });
    const { calls } = renderConfigPage(config);

    const card = getValetCard();

    // Activar el switch
    const switchEl = within(card).getByRole("switch");
    fireEvent.click(switchEl);

    const urlInput = within(card).getByPlaceholderText(
      "https://valet.example.com",
    );
    const tokenInput = within(card).getByPlaceholderText("val_...");
    fireEvent.change(urlInput, { target: { value: "https://valet.test" } });
    fireEvent.change(tokenInput, { target: { value: "val_abc" } });

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar valet/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        valet_url: "https://valet.test",
        valet_token: "val_abc",
      });
    });
  });

  it("envía cadenas vacías al guardar con el switch apagado", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
      valet_token: "val_abc",
    });
    const { calls } = renderConfigPage(config);

    const card = getValetCard();

    // Apagar el switch
    const switchEl = within(card).getByRole("switch");
    fireEvent.click(switchEl);

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar valet/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        valet_url: "",
        valet_token: "",
      });
    });
  });

  it("hace POST /api/notifications/test con channel valet", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
      valet_token: "val_abc",
    });
    const { calls } = renderConfigPage(config);

    const card = getValetCard();

    fireEvent.click(within(card).getByRole("button", { name: /test/i }));

    await waitFor(() => {
      const post = calls.find((c) => c.url === "/api/notifications/test");
      expect(post).toBeDefined();
      expect(post!.method).toBe("POST");
      expect(post!.body).toEqual({ channel: "valet" });
    });
  });
});
