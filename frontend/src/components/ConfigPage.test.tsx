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
    telegram_chat_id: null,
    matrix_configured: false,
    matrix_homeserver: null,
    matrix_room: null,
    webhook_configured: false,
    valet_configured: false,
    valet_url: null,
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

function getCard(title: string): HTMLElement {
  const t = screen.getByText(title);
  const card = t.closest(".ant-card");
  if (!card) throw new Error(`${title} card not found`);
  return card as HTMLElement;
}

function getValetCard(): HTMLElement {
  return getCard("🪐 Valet");
}

describe("ConfigPage — tarjeta Valet", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("no precarga el token: campo vacío aunque valet_configured sea true", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("🪐 Valet")).toBeInTheDocument();
    });

    const card = getValetCard();

    // URL visible in the (plain) input
    expect(within(card).getByDisplayValue("https://valet.test")).toBeInTheDocument();

    // Token field is empty and hints that one is already configured.
    const tokenInput = within(card).getByLabelText("Ingest key de Valet");
    expect(tokenInput).toHaveValue("");
    expect(tokenInput).toHaveAttribute("placeholder", "•••• configurado");
  });

  it("expone nombre accesible en el switch y los campos (a11y)", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
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

  it("omite valet_token al guardar con el campo vacío y el canal activado", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
    });
    const { calls } = renderConfigPage(config);

    const card = getValetCard();

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar valet/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({ valet_url: "https://valet.test" });
      expect(put!.body).not.toHaveProperty("valet_token");
    });
  });

  it("envía valet_token vacío al guardar con el switch apagado", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
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

  it("no envía un token de solo espacios (Valet)", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
    });
    const { calls } = renderConfigPage(config);

    const card = getValetCard();

    fireEvent.change(within(card).getByLabelText("Ingest key de Valet"), {
      target: { value: "   " },
    });

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar valet/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).not.toHaveProperty("valet_token");
    });
  });

  it("limpia el campo de token tras un guardado correcto (Valet)", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
    });
    const { calls } = renderConfigPage(config);

    const card = getValetCard();

    const tokenInput = within(card).getByLabelText("Ingest key de Valet");
    fireEvent.change(tokenInput, { target: { value: "val_new" } });

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar valet/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put!.body).toMatchObject({ valet_token: "val_new" });
    });

    await waitFor(() => {
      expect(within(card).getByLabelText("Ingest key de Valet")).toHaveValue("");
    });
  });

  it("hace POST /api/notifications/test con channel valet", async () => {
    const config = makeConfig({
      valet_configured: true,
      valet_url: "https://valet.test",
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

describe("ConfigPage — tarjeta Telegram", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("no precarga el token y omite telegram_token si el campo queda vacío", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    const tokenInput = within(card).getByPlaceholderText("•••• configurado");
    expect(tokenInput).toHaveValue("");

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar telegram/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).not.toHaveProperty("telegram_token");
      expect(put!.body).toMatchObject({ telegram_chat_id: "-100123" });
    });
  });

  it("envía el token nuevo al escribirlo y activar el canal", async () => {
    const config = makeConfig({ telegram_configured: false });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    const switchEl = within(card).getByRole("switch");
    fireEvent.click(switchEl);

    fireEvent.change(
      within(card).getByPlaceholderText("123456:ABC-DEF..."),
      { target: { value: "123:newtoken" } },
    );

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar telegram/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({ telegram_token: "123:newtoken" });
    });
  });

  it("envía telegram_token y telegram_chat_id vacíos al guardar con el switch apagado", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    // Apagar el switch
    fireEvent.click(within(card).getByRole("switch"));

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar telegram/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        telegram_token: "",
        telegram_chat_id: "",
      });
    });
  });

  it("limpia el campo de token tras un guardado correcto (Telegram)", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    const tokenInput = within(card).getByPlaceholderText("•••• configurado");
    fireEvent.change(tokenInput, { target: { value: "123:newtoken" } });

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar telegram/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put!.body).toMatchObject({ telegram_token: "123:newtoken" });
    });

    await waitFor(() => {
      expect(
        within(card).getByPlaceholderText("•••• configurado"),
      ).toHaveValue("");
    });
  });
});

describe("ConfigPage — tarjeta Matrix", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("no precarga el token y omite matrix_token si el campo queda vacío", async () => {
    const config = makeConfig({
      matrix_configured: true,
      matrix_homeserver: "https://matrix.test",
      matrix_room: "!room:matrix.test",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    const tokenInput = within(card).getByPlaceholderText("•••• configurado");
    expect(tokenInput).toHaveValue("");

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar matrix/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).not.toHaveProperty("matrix_token");
      expect(put!.body).toMatchObject({
        matrix_homeserver: "https://matrix.test",
        matrix_room: "!room:matrix.test",
      });
    });
  });

  it("envía matrix_token al escribir uno nuevo y activar el canal", async () => {
    const config = makeConfig({ matrix_configured: false });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    fireEvent.click(within(card).getByRole("switch"));

    fireEvent.change(
      within(card).getByPlaceholderText("https://matrix.example.com"),
      { target: { value: "https://matrix.test" } },
    );
    fireEvent.change(within(card).getByPlaceholderText("syt_..."), {
      target: { value: "syt_new" },
    });

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar matrix/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        matrix_homeserver: "https://matrix.test",
        matrix_token: "syt_new",
      });
    });
  });

  it("envía matrix_token vacío al guardar con el switch apagado", async () => {
    const config = makeConfig({
      matrix_configured: true,
      matrix_homeserver: "https://matrix.test",
      matrix_room: "!room:matrix.test",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    fireEvent.click(within(card).getByRole("switch"));

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar matrix/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        matrix_homeserver: "",
        matrix_token: "",
        matrix_room: "",
      });
    });
  });
});
