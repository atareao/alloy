import { describe, it, expect, vi, beforeEach, beforeAll } from "vitest";
import { useState } from "react";
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
    telegram_token_set: false,
    telegram_chat_id: null,
    telegram_topic_id: null,
    matrix_configured: false,
    matrix_token_set: false,
    matrix_homeserver: null,
    matrix_room: null,
    webhook_configured: false,
    valet_configured: false,
    valet_token_set: false,
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
            cron: "0 6 * * *",
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

  it("expone nombre accesible en el switch y el campo de token (a11y)", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    expect(
      within(card).getByRole("switch", { name: "Activar Telegram" }),
    ).toBeInTheDocument();
    expect(
      within(card).getByLabelText("Token del bot de Telegram"),
    ).toBeInTheDocument();
  });
});

describe("ConfigPage — tarjeta Telegram: Topic ID (opcional)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("precarga el campo Topic ID con el valor guardado", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
      telegram_topic_id: "42",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");
    const topicInput = within(card).getByLabelText(/topic id/i);
    expect(topicInput).toHaveValue("42");
  });

  it("incluye telegram_topic_id en el PUT al guardar la tarjeta Telegram", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
      telegram_topic_id: "42",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar telegram/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        telegram_chat_id: "-100123",
        telegram_topic_id: "42",
      });
    });
  });

  it("envía telegram_topic_id vacío al guardar con el switch apagado", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
      telegram_topic_id: "42",
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
      expect(put!.body).toMatchObject({ telegram_topic_id: "" });
    });
  });

  it("envía telegram_topic_id vacío cuando el campo queda vacío (borra el topic)", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
      telegram_topic_id: "42",
    });
    const { calls } = renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    fireEvent.change(within(card).getByLabelText(/topic id/i), {
      target: { value: "   " },
    });

    fireEvent.click(
      within(card).getByRole("button", { name: /guardar telegram/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      const body = put!.body as Record<string, unknown>;
      // "" borra el topic; null sería interpretado como "conservar".
      expect(body.telegram_topic_id).toBe("");
    });
  });

  it("sanea el campo Topic ID para aceptar solo dígitos", async () => {
    const config = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100123",
      telegram_topic_id: null,
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");
    const topicInput = within(card).getByLabelText(/topic id/i);

    fireEvent.change(topicInput, { target: { value: "4a2b-3" } });

    expect(topicInput).toHaveValue("423");
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

  it("expone nombre accesible en el switch y el campo de token (a11y)", async () => {
    const config = makeConfig({
      matrix_configured: true,
      matrix_homeserver: "https://matrix.test",
      matrix_room: "!room:matrix.test",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    expect(
      within(card).getByRole("switch", { name: "Activar Matrix" }),
    ).toBeInTheDocument();
    expect(
      within(card).getByLabelText("Access token de Matrix"),
    ).toBeInTheDocument();
  });
});

describe("ConfigPage — aviso de configuración incompleta", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("Matrix parcial (homeserver sin token): switch activado, campos visibles y aviso", async () => {
    const config = makeConfig({
      matrix_configured: false,
      matrix_token_set: false,
      matrix_homeserver: "https://matrix.test",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    expect(
      within(card).getByRole("switch", { name: "Activar Matrix" }),
    ).toBeChecked();
    expect(
      within(card).getByLabelText("Access token de Matrix"),
    ).toBeInTheDocument();
    expect(
      within(card).getByText("Faltan campos para completar la configuración"),
    ).toBeInTheDocument();
  });

  it("Matrix completa (configured=true): no muestra aviso", async () => {
    const config = makeConfig({
      matrix_configured: true,
      matrix_homeserver: "https://matrix.test",
      matrix_room: "!room:matrix.test",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    expect(
      within(card).getByRole("switch", { name: "Activar Matrix" }),
    ).toBeChecked();
    expect(
      within(card).queryByText("Faltan campos para completar la configuración"),
    ).not.toBeInTheDocument();
  });

  it("token guardado sin canal completo (token_set=true, configured=false): switch activado y aviso", async () => {
    const config = makeConfig({
      matrix_configured: false,
      matrix_token_set: true,
      matrix_homeserver: null,
      matrix_room: null,
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    expect(
      within(card).getByRole("switch", { name: "Activar Matrix" }),
    ).toBeChecked();
    expect(
      within(card).getByText("Faltan campos para completar la configuración"),
    ).toBeInTheDocument();
  });

  it("Telegram parcial (chat_id sin token): switch activado y aviso", async () => {
    const config = makeConfig({
      telegram_configured: false,
      telegram_token_set: false,
      telegram_chat_id: "-100123",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    expect(
      within(card).getByRole("switch", { name: "Activar Telegram" }),
    ).toBeChecked();
    expect(
      within(card).getByText("Faltan campos para completar la configuración"),
    ).toBeInTheDocument();
  });

  it("Valet parcial (url sin token): switch activado y aviso", async () => {
    const config = makeConfig({
      valet_configured: false,
      valet_token_set: false,
      valet_url: "https://valet.test",
    });
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("🪐 Valet")).toBeInTheDocument();
    });

    const card = getValetCard();

    expect(
      within(card).getByRole("switch", { name: "Activar Valet" }),
    ).toBeChecked();
    expect(
      within(card).getByText("Faltan campos para completar la configuración"),
    ).toBeInTheDocument();
  });

  it("activar manualmente un canal totalmente vacío: switch on, campos visibles y aviso", async () => {
    // Canal sin campos y sin configurar: el aviso debe aparecer al activarlo.
    const config = makeConfig();
    renderConfigPage(config);

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    const card = getCard("💬 Matrix");

    const switchEl = within(card).getByRole("switch", {
      name: "Activar Matrix",
    });
    expect(switchEl).not.toBeChecked();

    fireEvent.click(switchEl);

    await waitFor(() => {
      expect(
        within(card).getByRole("switch", { name: "Activar Matrix" }),
      ).toBeChecked();
    });
    expect(
      within(card).getByLabelText("Access token de Matrix"),
    ).toBeInTheDocument();
    expect(
      within(card).getByText("Faltan campos para completar la configuración"),
    ).toBeInTheDocument();
  });

  it("el aviso desaparece tras un guardado que devuelve configured=true", async () => {
    const initial = makeConfig({ matrix_homeserver: "https://matrix.test" });
    const saved = makeConfig({
      matrix_configured: true,
      matrix_homeserver: "https://matrix.test",
      matrix_room: "!room:matrix.test",
    });

    // Mount-time requests plus a PUT /api/config that returns a fully
    // configured channel (like the real backend after a complete save).
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      if (url === "/api/update-check/config") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "0 6 * * *",
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
        return Promise.resolve(new Response(JSON.stringify(saved)));
      }
      return Promise.resolve(new Response("ok"));
    });
    vi.stubGlobal("fetch", mock);

    // Stateful harness so setConfig (called by the page on save) feeds the new
    // config back through the `config` prop, as App.tsx does.
    function Harness() {
      const [cfg, setCfg] = useState<AppConfig>(initial);
      return (
        <ConfigPage
          config={cfg}
          setConfig={setCfg}
          colorScheme="dark"
          setColorScheme={() => {}}
        />
      );
    }

    render(
      <Wrapper>
        <Harness />
      </Wrapper>,
    );

    await waitFor(() => {
      expect(screen.getByText("💬 Matrix")).toBeInTheDocument();
    });

    expect(
      within(getCard("💬 Matrix")).getByText(
        "Faltan campos para completar la configuración",
      ),
    ).toBeInTheDocument();

    fireEvent.click(
      within(getCard("💬 Matrix")).getByRole("button", {
        name: /guardar matrix/i,
      }),
    );

    await waitFor(() => {
      expect(
        within(getCard("💬 Matrix")).queryByText(
          "Faltan campos para completar la configuración",
        ),
      ).not.toBeInTheDocument();
    });
  });
});

describe("ConfigPage — tarjeta Exportar / Importar", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("advierte de que los tokens de notificación no se exportan", async () => {
    renderConfigPage(makeConfig());

    fireEvent.click(screen.getByText("ℹ️ Información"));

    expect(
      await screen.findByText("📦 Exportar / Importar configuración"),
    ).toBeInTheDocument();

    expect(
      screen.getByText(
        /tokens de notificación \(Telegram, Matrix, Valet\) y la URL del webhook no se exportan por seguridad/i,
      ),
    ).toBeInTheDocument();
  });
});

/**
 * Consistencia de defaults backend↔UI (change reconcile-update-defaults).
 *
 * El formulario debe arrancar con los valores canónicos `0 6 * * *` (cron) y
 * `600` (timeout de pull) antes de que resuelva el GET /api/update-check/config,
 * y la etiqueta del timeout debe indicar `600 (10 min)`.
 */
describe("ConfigPage — defaults canónicos de updates", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  // Los campos de cron/timeout solo se montan con la revisión activada. El GET
  // devuelve los canónicos del backend, pero el placeholder y la etiqueta del
  // timeout son estáticos: deben coincidir aunque la carga esté en vuelo.
  function renderWithEnabledUpdates() {
    const mock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/update-check/config") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "0 6 * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 600,
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
      return Promise.resolve(new Response("ok"));
    });
    vi.stubGlobal("fetch", mock);

    render(
      <Wrapper>
        <ConfigPage
          config={makeConfig()}
          setConfig={vi.fn()}
          colorScheme="dark"
          setColorScheme={() => {}}
        />
      </Wrapper>,
    );
  }

  it("usa el cron canónico 0 6 * * * como placeholder inicial", async () => {
    renderWithEnabledUpdates();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    expect(
      await screen.findByPlaceholderText("0 6 * * *"),
    ).toBeInTheDocument();
    expect(
      screen.queryByPlaceholderText("0 */6 * * *"),
    ).not.toBeInTheDocument();
  });

  it("muestra el timeout por defecto canónico 600 (10 min)", async () => {
    renderWithEnabledUpdates();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    expect(
      await screen.findByText(/Default: 600 \(10 min\)/),
    ).toBeInTheDocument();
    expect(screen.queryByText(/1800/)).not.toBeInTheDocument();
  });

  it("vaciar el timeout no cambia el default mostrado (600)", async () => {
    renderWithEnabledUpdates();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // El campo arranca precargado con el canónico 600.
    const timeoutInput = await screen.findByDisplayValue("600");

    // El usuario lo cambia a un valor no-default…
    fireEvent.change(timeoutInput, { target: { value: "900" } });
    expect(screen.getByDisplayValue("900")).toBeInTheDocument();

    // …y al vaciarlo se aplica el fallback canónico 600 (no 1800).
    fireEvent.change(screen.getByDisplayValue("900"), {
      target: { value: "" },
    });

    expect(screen.getByDisplayValue("600")).toBeInTheDocument();
    expect(screen.queryByDisplayValue("1800")).not.toBeInTheDocument();
  });
});

/**
 * RED (task 1.14) — guarda de arranque (update-rollback-guard).
 *
 * Contrato que deben cumplir estos tests (aún NO implementado):
 *  - ConfigPage renderiza los tres settings globales de la guarda con `aria-label`:
 *      "Máximo de intentos de arranque"        → `update_max_start_attempts`
 *      "Intervalo entre intentos (s)"          → `update_start_attempt_interval_secs`
 *      "Ventana de salud (s)"                  → `update_health_window_secs`
 *  - Los valores del `GET /api/update-check/config` se precargan en los campos.
 *  - Al guardar la revisión, el `PUT /api/update-check/config` incluye los tres campos.
 *
 * En RED fallan porque los campos no existen en ConfigPage.
 */
describe("ConfigPage — guarda de arranque (update-rollback-guard)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function installGuardFetchMock(): FetchCall[] {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "0 6 * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 600,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 5,
              update_health_window_secs: 30,
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
      if (url === "/api/update-check/config" && method === "PUT") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "0 6 * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              ...body,
            }),
          ),
        );
      }
      return Promise.resolve(new Response("ok"));
    });
    vi.stubGlobal("fetch", mock);
    return calls;
  }

  function renderGuardPage() {
    const config = makeConfig();
    const setConfig = vi.fn();
    const calls = installGuardFetchMock();
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
    return { calls };
  }

  it("renderiza los tres campos globales de la guarda en la pestaña de actualizaciones", async () => {
    renderGuardPage();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    expect(
      await screen.findByLabelText("Máximo de intentos de arranque"),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText("Intervalo entre intentos (s)"),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Ventana de salud (s)")).toBeInTheDocument();
  });

  it("precarga en los campos los valores del GET /api/update-check/config", async () => {
    renderGuardPage();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    await waitFor(() =>
      expect(
        screen.getByLabelText("Máximo de intentos de arranque"),
      ).toHaveValue("3"),
    );
    await waitFor(() =>
      expect(screen.getByLabelText("Intervalo entre intentos (s)")).toHaveValue(
        "5",
      ),
    );
    await waitFor(() =>
      expect(screen.getByLabelText("Ventana de salud (s)")).toHaveValue("30"),
    );
  });

  it("incluye los tres campos en el PUT al guardar la revisión", async () => {
    const { calls } = renderGuardPage();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // Espera a que la precarga asíncrona del GET se aplique antes de editar.
    await waitFor(() =>
      expect(
        screen.getByLabelText("Máximo de intentos de arranque"),
      ).toHaveValue("3"),
    );
    fireEvent.change(screen.getByLabelText("Máximo de intentos de arranque"), {
      target: { value: "5" },
    });
    fireEvent.change(screen.getByLabelText("Intervalo entre intentos (s)"), {
      target: { value: "10" },
    });
    fireEvent.change(screen.getByLabelText("Ventana de salud (s)"), {
      target: { value: "45" },
    });

    fireEvent.click(
      screen.getByRole("button", { name: /guardar revisión/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-check/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        update_max_start_attempts: 5,
        update_start_attempt_interval_secs: 10,
        update_health_window_secs: 45,
      });
    });
  });
});

/**
 * `validate-cron-on-put` — el backend valida el cron y responde 400 con
 * `{"error": ...}`. La UI debe mostrar el mensaje del backend, no uno genérico.
 *
 * En RED falla: `saveUpdateCheck` ignora el body del error y muestra siempre
 * "Error al guardar la revisión de actualizaciones".
 */
describe("ConfigPage — error de cron inválido desde el backend", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function installInvalidCronFetchMock() {
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";

      if (url === "/api/update-check/config" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              // Distinto del default de `useState("0 6 * * *")` para que la
              // aserción de precarga sea realmente probatoria (#134).
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 600,
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
      if (url === "/api/update-check/config" && method === "PUT") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              error: "expresión cron inválida: se esperaban 5 campos",
            }),
            { status: 400 },
          ),
        );
      }
      return Promise.resolve(new Response("ok"));
    });
    vi.stubGlobal("fetch", mock);
  }

  function renderInvalidCronPage() {
    installInvalidCronFetchMock();
    render(
      <Wrapper>
        <ConfigPage
          config={makeConfig()}
          setConfig={vi.fn()}
          colorScheme="dark"
          setColorScheme={() => {}}
        />
      </Wrapper>,
    );
  }

  it("muestra el mensaje de error del backend (no el genérico) cuando el cron es inválido", async () => {
    renderInvalidCronPage();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // Espera la precarga asíncrona del GET antes de editar (#134). El valor
    // del mock ("*/30 * * * *") difiere del initial state, así que esta
    // aserción fallaría si el preload no aplicara.
    const cronInput = await screen.findByPlaceholderText("0 6 * * *");
    await waitFor(() => expect(cronInput).toHaveValue("*/30 * * * *"));

    fireEvent.change(cronInput, { target: { value: "not a cron" } });
    fireEvent.click(screen.getByRole("button", { name: /guardar revisión/i }));

    expect(
      await screen.findByText("expresión cron inválida: se esperaban 5 campos"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Error al guardar la revisión de actualizaciones"),
    ).not.toBeInTheDocument();
  });
});

/**
 * Auditoría (Hallazgo 2): faltaban los overrides por política
 * (`max_start_attempts`, `health_window_secs`) en la tarjeta de política por
 * defecto. Deben renderizarse, precargarse desde el GET y enviarse SIEMPRE en
 * el PUT para no resetear overrides existentes.
 */
describe("ConfigPage — overrides de la guarda en la política por defecto", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function installDefaultPolicyMock(): FetchCall[] {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "0 6 * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
            }),
          ),
        );
      }
      if (url === "/api/update-policies/default" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              action: "pull-restart",
              cleanup_old_image: false,
              rollback_on_failure: false,
              max_start_attempts: 4,
              health_window_secs: 20,
            }),
          ),
        );
      }
      if (url === "/api/update-policies/default" && method === "PUT") {
        return Promise.resolve(new Response(JSON.stringify({ ...body })));
      }
      return Promise.resolve(new Response("ok"));
    });
    vi.stubGlobal("fetch", mock);
    return calls;
  }

  function renderDefaultPolicyPage() {
    const calls = installDefaultPolicyMock();
    render(
      <Wrapper>
        <ConfigPage
          config={makeConfig()}
          setConfig={vi.fn()}
          colorScheme="dark"
          setColorScheme={() => {}}
        />
      </Wrapper>,
    );
    return { calls };
  }

  it("renderiza los overrides y precarga sus valores desde GET /api/update-policies/default", async () => {
    renderDefaultPolicyPage();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    await waitFor(() =>
      expect(
        screen.getByLabelText("Override máximo de intentos de arranque"),
      ).toHaveValue("4"),
    );
    await waitFor(() =>
      expect(
        screen.getByLabelText("Override ventana de salud (s)"),
      ).toHaveValue("20"),
    );
  });

  it("incluye max_start_attempts y health_window_secs en el PUT de la política por defecto", async () => {
    const { calls } = renderDefaultPolicyPage();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // Espera a que la precarga asíncrona del GET se aplique antes de editar.
    await waitFor(() =>
      expect(
        screen.getByLabelText("Override máximo de intentos de arranque"),
      ).toHaveValue("4"),
    );
    fireEvent.change(
      screen.getByLabelText("Override máximo de intentos de arranque"),
      { target: { value: "6" } },
    );
    fireEvent.change(screen.getByLabelText("Override ventana de salud (s)"), {
      target: { value: "15" },
    });

    fireEvent.click(
      screen.getByRole("button", { name: /guardar política por defecto/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-policies/default" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        max_start_attempts: 6,
        health_window_secs: 15,
      });
    });
  });
});
