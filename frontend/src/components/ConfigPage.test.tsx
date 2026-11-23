import { describe, it, expect, vi, beforeEach, beforeAll } from "vitest";
import { useEffect, useState } from "react";
import {
  act,
  render,
  screen,
  fireEvent,
  waitFor,
  within,
} from "@testing-library/react";
import { ConfigProvider } from "antd";
import ConfigPage from "./ConfigPage";
import { CRON_PRESETS } from "../cronPresets";
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

/**
 * Activa una sub-pestaña vertical (antd `Tabs tabPosition="left"`) y devuelve la
 * Card cuyo título coincide con `cardName`.
 *
 * Desde la migración a pestañas verticales, las Cards de canal viven dentro de
 * su propia pestaña y antd monta los paneles en diferido: el contenido de una
 * pestaña inactiva no está en el DOM. Por eso primero se pulsa la pestaña
 * (`role="tab"`) y después se busca la Card. El título lleva emoji, así que se
 * localiza con un matcher por regex sobre el `heading`, no con string exacto.
 */
async function openSubTabCard(
  tabName: RegExp,
  cardName: RegExp,
): Promise<HTMLElement> {
  fireEvent.click(screen.getByRole("tab", { name: tabName }));
  const heading = await screen.findByRole("heading", { name: cardName });
  const card = heading.closest(".ant-card");
  if (!card) throw new Error(`Card "${cardName}" not found`);
  return card as HTMLElement;
}

async function getValetCard(): Promise<HTMLElement> {
  return openSubTabCard(/^valet$/i, /valet/i);
}

async function getMatrixCard(): Promise<HTMLElement> {
  return openSubTabCard(/^matrix$/i, /matrix/i);
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

    // La Card de Valet vive en su sub-pestaña: activarla primero.
    const card = await getValetCard();

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

    // La Card de Valet vive en su sub-pestaña: activarla primero.
    const card = await getValetCard();

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

    const card = await getValetCard();

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

    const card = await getValetCard();

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

    const card = await getValetCard();

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

    const card = await getValetCard();

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

    const card = await getValetCard();

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

    const card = await getValetCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Valet vive en su sub-pestaña: activarla primero.
    const card = await getValetCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const card = await getMatrixCard();

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

    // La Card de Matrix vive en su sub-pestaña: activarla primero.
    const matrixCard = await getMatrixCard();

    expect(
      within(matrixCard).getByText(
        "Faltan campos para completar la configuración",
      ),
    ).toBeInTheDocument();

    fireEvent.click(
      within(matrixCard).getByRole("button", {
        name: /guardar matrix/i,
      }),
    );

    await waitFor(() => {
      expect(
        within(matrixCard).queryByText(
          "Faltan campos para completar la configuración",
        ),
      ).not.toBeInTheDocument();
    });
  });
});

describe("ConfigPage — sincronización desde props", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("precarga los campos y vacía el token al recibir una configuración nueva", async () => {
    const initial = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100111",
      telegram_topic_id: "1",
    });
    const updated = makeConfig({
      telegram_configured: true,
      telegram_chat_id: "-100222",
      telegram_topic_id: "2",
    });
    installFetchMock(initial);

    // Harness con estado propio: simula a App.tsx reenviando la config por la
    // prop tras el fetch eager (o tras un guardado).
    const pushConfig: { current: ((c: AppConfig) => void) | null } = {
      current: null,
    };
    function Harness() {
      const [cfg, setCfg] = useState<AppConfig>(initial);
      useEffect(() => {
        pushConfig.current = setCfg;
      }, []);
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
      expect(screen.getByText("📱 Telegram")).toBeInTheDocument();
    });

    const card = getCard("📱 Telegram");

    // Precarga desde la prop inicial.
    expect(
      within(card).getByPlaceholderText("-1001234567890"),
    ).toHaveValue("-100111");
    expect(within(card).getByLabelText("Topic ID (opcional)")).toHaveValue("1");

    // El usuario teclea un token nuevo…
    const tokenInput = within(card).getByLabelText(
      "Token del bot de Telegram",
    );
    fireEvent.change(tokenInput, { target: { value: "123:typed" } });
    expect(tokenInput).toHaveValue("123:typed");

    // …y llega una configuración nueva por props.
    act(() => pushConfig.current!(updated));

    await waitFor(() => {
      expect(
        within(card).getByPlaceholderText("-1001234567890"),
      ).toHaveValue("-100222");
    });
    expect(within(card).getByLabelText("Topic ID (opcional)")).toHaveValue("2");
    expect(
      within(card).getByLabelText("Token del bot de Telegram"),
    ).toHaveValue("");
    expect(
      within(card).getByRole("switch", { name: "Activar Telegram" }),
    ).toBeChecked();
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
 * Consistencia de defaults backend↔UI (change expose-update-defaults-api).
 *
 * El frontend NO debe hardcodear los defaults: el placeholder del cron, la
 * etiqueta del timeout y el fallback al vaciar `pull_timeout_secs` deben salir
 * de `GET /api/update-check/defaults`. Para que el test sea probatorio, el mock
 * devuelve valores **distintos** de los literales históricos (`0 6 * * *` /
 * `600`): `cron: "15 3 * * *"` y `pull_timeout_secs: 900`. La carga del GET de
 * config aporta además valores diferentes, de modo que si el frontend siguiera
 * hardcodeando, las aserciones fallarían.
 */
describe("ConfigPage — defaults desde la API (GET /api/update-check/defaults)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  // Los campos de cron/timeout solo se montan con la revisión activada.
  function renderWithEnabledUpdates() {
    const mock = vi.fn().mockImplementation((url: string) => {
      if (url === "/api/update-check/config") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              // Distintos de los defaults de la API para que la aserción de
              // fallback (900) no coincida por casualidad con la precarga.
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 480,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "15 3 * * *",
              pull_timeout_secs: 900,
              check_interval_ms: 2000,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 10,
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

  // Variante donde el GET de defaults falla: el placeholder debe quedar vacío
  // (sin parpadeo) y vaciar el timeout NO debe caer a 0 (fuera de min={60}).
  function renderWithDefaultsUnavailable(): FetchCall[] {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 480,
            }),
          ),
        );
      }
      if (url === "/api/update-check/config" && method === "PUT") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              ...body,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        return Promise.resolve(new Response("boom", { status: 500 }));
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
    return calls;
  }

  it("sin defaults cargados, el placeholder del cron queda vacío (sin parpadeo)", async () => {
    renderWithDefaultsUnavailable();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // Se espera a que los GET de montaje se apliquen; el placeholder no debe
    // mostrar el literal histórico.
    await screen.findByDisplayValue("*/30 * * * *");
    expect(screen.queryByPlaceholderText("0 6 * * *")).not.toBeInTheDocument();
  });

  it("al vaciar el timeout sin defaults cargados conserva el valor previo (no cae a 0)", async () => {
    const calls = renderWithDefaultsUnavailable();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    const timeoutInput = await screen.findByDisplayValue("480");
    fireEvent.change(timeoutInput, { target: { value: "1200" } });

    // Vaciar sin default disponible: no debe aplicarse 0 (queda por debajo de
    // min={60}); se conserva el valor previo en el estado del formulario.
    fireEvent.change(screen.getByDisplayValue("1200"), {
      target: { value: "" },
    });
    fireEvent.blur(timeoutInput);
    // El input del timeout conserva el valor previo; no cae a 0.
    expect((timeoutInput as HTMLInputElement).value).toBe("1200");

    // Al guardar, el valor previo (1200) se envía tal cual, no 0.
    fireEvent.click(screen.getByRole("button", { name: /guardar revisión/i }));
    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-check/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({ pull_timeout_secs: 1200 });
    });
  });

  it("usa el cron devuelto por la API como placeholder (no el literal)", async () => {
    renderWithEnabledUpdates();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    expect(
      await screen.findByPlaceholderText("15 3 * * *"),
    ).toBeInTheDocument();
    expect(
      screen.queryByPlaceholderText("0 6 * * *"),
    ).not.toBeInTheDocument();
  });

  it("muestra el timeout por defecto devuelto por la API (900 → 15 min)", async () => {
    renderWithEnabledUpdates();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    expect(
      await screen.findByText(/Default: 900 \(15 min\)/),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Default: 600 \(10 min\)/)).not.toBeInTheDocument();
  });

  it("al vaciar el timeout aplica el default de la API (900), no el literal 600", async () => {
    renderWithEnabledUpdates();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // El campo arranca precargado con el valor del GET /config (480), distinto
    // del default de la API.
    const timeoutInput = await screen.findByDisplayValue("480");

    // El usuario lo cambia a un valor no-default…
    fireEvent.change(timeoutInput, { target: { value: "1200" } });
    expect(screen.getByDisplayValue("1200")).toBeInTheDocument();

    // …y al vaciarlo se aplica el fallback devuelto por la API (900), no el
    // literal histórico 600.
    fireEvent.change(screen.getByDisplayValue("1200"), {
      target: { value: "" },
    });

    expect(screen.getByDisplayValue("900")).toBeInTheDocument();
    expect(screen.queryByDisplayValue("600")).not.toBeInTheDocument();
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
 * `expose-guard-defaults-api` — el fallback al vaciar los campos de la guarda
 * debe salir de `GET /api/update-check/defaults`, no de los literales `3/10/30`.
 *
 * El mock devuelve guard defaults **distintos** de los canónicos
 * (`5/20/45`) y la precarga del `GET /config` valores también distintos
 * (`7/33/90`), de modo que la aserción del fallback es probatoria: si el
 * frontend siguiera usando los literales, el valor aplicado sería `3/10/30`.
 */
describe("ConfigPage — la guarda usa los defaults de la API al vaciar", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function renderGuardDefaults(): FetchCall[] {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 480,
              // Distintos de los canónicos (3/10/30) y de los defaults de la API
              // (5/20/45) para que el fallback sea realmente probatorio.
              update_max_start_attempts: 7,
              update_start_attempt_interval_secs: 33,
              update_health_window_secs: 90,
            }),
          ),
        );
      }
      if (url === "/api/update-check/config" && method === "PUT") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              ...body,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "15 3 * * *",
              pull_timeout_secs: 900,
              check_interval_ms: 2000,
              update_max_start_attempts: 5,
              update_start_attempt_interval_secs: 20,
              update_health_window_secs: 45,
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
    return calls;
  }

  it("vaciar 'Máximo de intentos de arranque' aplica el default de la API (5), no el literal 3", async () => {
    const calls = renderGuardDefaults();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    const input = await screen.findByLabelText("Máximo de intentos de arranque");
    // La precarga del GET /config deja el valor previo 7.
    await waitFor(() => expect(input).toHaveValue("7"));

    fireEvent.change(input, { target: { value: "9" } });
    expect(input).toHaveValue("9");

    // Al vaciarlo, el fallback es el de la API (5), no el literal 3.
    fireEvent.change(input, { target: { value: "" } });
    expect(input).toHaveValue("5");
    expect(input).not.toHaveValue("3");

    fireEvent.click(
      screen.getByRole("button", { name: /guardar revisión/i }),
    );
    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-check/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({ update_max_start_attempts: 5 });
    });
  });

  it("vaciar 'Intervalo entre intentos (s)' aplica el default de la API (20), no el literal 10", async () => {
    const calls = renderGuardDefaults();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    const input = await screen.findByLabelText("Intervalo entre intentos (s)");
    await waitFor(() => expect(input).toHaveValue("33"));

    fireEvent.change(input, { target: { value: "77" } });
    expect(input).toHaveValue("77");

    fireEvent.change(input, { target: { value: "" } });
    expect(input).toHaveValue("20");
    expect(input).not.toHaveValue("10");

    fireEvent.click(
      screen.getByRole("button", { name: /guardar revisión/i }),
    );
    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-check/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        update_start_attempt_interval_secs: 20,
      });
    });
  });

  it("vaciar 'Ventana de salud (s)' aplica el default de la API (45), no el literal 30", async () => {
    const calls = renderGuardDefaults();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    const input = await screen.findByLabelText("Ventana de salud (s)");
    await waitFor(() => expect(input).toHaveValue("90"));

    fireEvent.change(input, { target: { value: "120" } });
    expect(input).toHaveValue("120");

    fireEvent.change(input, { target: { value: "" } });
    expect(input).toHaveValue("45");
    expect(input).not.toHaveValue("30");

    fireEvent.click(
      screen.getByRole("button", { name: /guardar revisión/i }),
    );
    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-check/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({ update_health_window_secs: 45 });
    });
  });
});

/**
 * `validate-cron-on-put` — el backend valida el cron y responde 400 con
 * `{"error": ...}`. La UI debe mostrar el mensaje del backend, no uno genérico.
 *
 * Nota: `harden-cron-client-validation` añade un pre-chequeo en el cliente que
 * bloquea los crons estructuralmente inválidos y los de valores fuera de rango
 * (p. ej. "not a cron" o "99 99 99 99 99") antes de enviar el PUT. Para seguir
 * ejercitando la ruta "el backend responde 400", se usa un cron que pasa el
 * pre-chequeo endurecido (`0 6 * * 1-5`) pero que el mock del backend rechaza.
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
      if (url === "/api/update-check/defaults") {
        // El placeholder del cron sale de la API; se devuelven los canónicos.
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "0 6 * * *",
              pull_timeout_secs: 600,
              check_interval_ms: 2000,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 10,
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

    // Cron client-valid (5 campos, en rango, con rango a-b): atraviesa el
    // pre-chequeo y es el backend quien responde 400 con su mensaje.
    fireEvent.change(cronInput, { target: { value: "0 6 * * 1-5" } });
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

    // Pestaña «Actualizaciones» → sub-pestaña «Política» (los paneles de antd
    // se montan en diferido, así que hay que activarla).
    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));
    fireEvent.click(screen.getByRole("tab", { name: /^política$/i }));

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
    fireEvent.click(screen.getByRole("tab", { name: /^política$/i }));

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

/**
 * `harden-cron-client-validation` — el formulario valida y normaliza el cron
 * ANTES de enviar el PUT, alineado con la gramática de la crate `cron` del
 * backend (autoridad final). Contrato:
 *  - 6 campos ⇒ se quita el campo de segundos (primero) al salir del campo;
 *    el PUT envía 5 campos.
 *  - Cada campo se valida por rangos (minuto 0–59, hora 0–23, día-del-mes
 *    1–31, mes 1–12, día-de-semana 1–7), listas `,`, rangos `a-b` no invertidos,
 *    pasos `1..=máximo` del campo, `?` (solo día-del-mes/día-de-semana y como
 *    único elemento) y nombres alfabéticos ≥3 letras (solo mes/día-de-semana).
 *  - Cron inválido ⇒ error inline + PUT NO disparado.
 *  - Cron válido de 5 campos ⇒ sin error + PUT disparado.
 */
describe("ConfigPage — validación y normalización del cron en el cliente", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function installCronFetchMock(): FetchCall[] {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

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
      if (url === "/api/update-check/config" && method === "PUT") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: body?.cron,
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              ...body,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        // Placeholder del cron distinto del valor precargado, para localizar el
        // Input por placeholder sin ambigüedad.
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "15 3 * * *",
              pull_timeout_secs: 900,
              check_interval_ms: 2000,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 10,
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
      return Promise.resolve(new Response("ok"));
    });
    vi.stubGlobal("fetch", mock);
    return calls;
  }

  async function renderAndOpenCronTab(): Promise<HTMLInputElement> {
    const calls = installCronFetchMock();
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

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // El campo solo se monta con `ucEnabled` resuelto por el GET de precarga,
    // y su valor debe ser el del backend (`*/30 * * * *`), no el initial state.
    const cronInput = (await screen.findByPlaceholderText(
      "15 3 * * *",
    )) as HTMLInputElement;
    await waitFor(() => expect(cronInput).toHaveValue("*/30 * * * *"));

    // Expone los calls vía el closure del mock devuelto por installCronFetchMock.
    (cronInput as unknown as { __calls: FetchCall[] }).__calls = calls;
    return cronInput;
  }

  function callsOf(cronInput: HTMLInputElement): FetchCall[] {
    return (cronInput as unknown as { __calls: FetchCall[] }).__calls;
  }

  function putCalls(calls: FetchCall[]): FetchCall[] {
    return calls.filter(
      (c) => c.url === "/api/update-check/config" && c.method === "PUT",
    );
  }

  it("normaliza un cron de 6 campos a 5 al salir del campo y guarda el valor normalizado", async () => {
    const cronInput = await renderAndOpenCronTab();
    const calls = callsOf(cronInput);

    fireEvent.change(cronInput, { target: { value: "0 0 6 * * *" } });
    fireEvent.blur(cronInput);

    // Al salir del campo quedan 5 campos (se descarta el de segundos).
    await waitFor(() => expect(cronInput).toHaveValue("0 6 * * *"));

    fireEvent.click(screen.getByRole("button", { name: /guardar revisión/i }));

    await waitFor(() => {
      const puts = putCalls(calls);
      expect(puts).toHaveLength(1);
      expect(puts[0].body).toMatchObject({ cron: "0 6 * * *" });
    });
  });

  it("un cron malformado muestra error inline y NO dispara el PUT", async () => {
    const cronInput = await renderAndOpenCronTab();
    const calls = callsOf(cronInput);

    fireEvent.change(cronInput, { target: { value: "not a cron" } });
    fireEvent.click(screen.getByRole("button", { name: /guardar revisión/i }));

    expect(
      await screen.findByText(/expresión cron inválida/i),
    ).toBeInTheDocument();
    expect(putCalls(calls)).toHaveLength(0);
  });

  it("un cron válido de 5 campos no muestra error y dispara el PUT", async () => {
    const cronInput = await renderAndOpenCronTab();
    const calls = callsOf(cronInput);

    fireEvent.change(cronInput, { target: { value: "0 6 * * *" } });
    fireEvent.click(screen.getByRole("button", { name: /guardar revisión/i }));

    await waitFor(() => {
      const puts = putCalls(calls);
      expect(puts).toHaveLength(1);
      expect(puts[0].body).toMatchObject({ cron: "0 6 * * *" });
    });
    expect(screen.queryByText(/expresión cron inválida/i)).not.toBeInTheDocument();
  });

  // Endurecimiento: el cliente valida rangos, listas, pasos (1..máximo) y
  // rangos no invertidos por campo, no solo la estructura de 5 campos. El
  // día-de-semana es 1–7 (la crate usa 1=domingo; `0` es inválido).
  it.each([
    "99 0 * * *",
    "0 0 * * 0",
    "30-10 * * * *",
    "*/0 * * * *",
    "*/60 * * * *",
  ])(
    "un cron client-inválido (%s) muestra error inline y NO dispara el PUT",
    async (value) => {
      const cronInput = await renderAndOpenCronTab();
      const calls = callsOf(cronInput);

      fireEvent.change(cronInput, { target: { value } });
      fireEvent.click(
        screen.getByRole("button", { name: /guardar revisión/i }),
      );

      expect(
        await screen.findByText(/expresión cron inválida/i),
      ).toBeInTheDocument();
      expect(putCalls(calls)).toHaveLength(0);
    },
  );

  it.each([
    "*/15 * * * *",
    "0 6 * * MON",
    "0 6 1 JAN *",
    "0 6 * * 1-5",
    "0 6 ? * *",
    "0 6 * * MON-FRI",
    "0 6 1,15 * *",
    "0 6 * * MONDAY",
    "0 6 1 JANUARY *",
    "0 0 * * SUN",
    "0 6 31 * *",
    "0 6 * 12 *",
  ])(
    "un cron client-válido (%s) no muestra error y dispara el PUT",
    async (value) => {
      const cronInput = await renderAndOpenCronTab();
      const calls = callsOf(cronInput);

      fireEvent.change(cronInput, { target: { value } });
      fireEvent.click(
        screen.getByRole("button", { name: /guardar revisión/i }),
      );

      await waitFor(() => {
        const puts = putCalls(calls);
        expect(puts).toHaveLength(1);
        expect(puts[0].body).toMatchObject({ cron: value });
      });
      expect(
        screen.queryByText(/expresión cron inválida/i),
      ).not.toBeInTheDocument();
    },
  );

  it("el preset «Cada domingo» usa día-de-semana SUN (1–7) y es client-válido", () => {
    // Regresión del preset roto: `0 0 * * 0` es inválido en la crate (1=domingo).
    expect(CRON_PRESETS.find((p) => p.label === "Cada domingo")?.value).toBe(
      "0 0 * * SUN",
    );
  });

  it("todos los presets de frecuencia pasan la validación del cliente", async () => {
    const cronInput = await renderAndOpenCronTab();

    for (const preset of CRON_PRESETS) {
      fireEvent.change(cronInput, { target: { value: preset.value } });
      fireEvent.blur(cronInput);

      await waitFor(() => expect(cronInput).toHaveValue(preset.value));
      expect(
        screen.queryByText(/expresión cron inválida/i),
      ).not.toBeInTheDocument();
    }
  });

  it("un cron con nombre de día (0 6 * * MON) no muestra error y dispara el PUT", async () => {
    const cronInput = await renderAndOpenCronTab();
    const calls = callsOf(cronInput);

    fireEvent.change(cronInput, { target: { value: "0 6 * * MON" } });
    fireEvent.blur(cronInput);
    fireEvent.click(screen.getByRole("button", { name: /guardar revisión/i }));

    await waitFor(() => {
      const puts = putCalls(calls);
      expect(puts).toHaveLength(1);
      expect(puts[0].body).toMatchObject({ cron: "0 6 * * MON" });
    });
    expect(screen.queryByText(/expresión cron inválida/i)).not.toBeInTheDocument();
  });

  it("con la revisión desactivada, un cron inválido muestra el error (no-op silencioso)", async () => {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config" && method === "GET") {
        // Desactivada + cron inválido: el campo cron NO se monta.
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "not a cron",
              enabled: false,
              notify: false,
              last_run_at: null,
              // Señal de que el GET de config se ha aplicado y renderizado:
              // el bloque "Próxima revisión" queda fuera del gating ucEnabled.
              next_run_at: "2026-11-22T06:00:00Z",
              pull_timeout_secs: 600,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "15 3 * * *",
              pull_timeout_secs: 900,
              check_interval_ms: 2000,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 10,
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
    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    // El campo cron NO se monta con la revisión desactivada (está bajo el
    // gating `ucEnabled`), así que no sirve como señal de carga. Se ancla a
    // "Próxima revisión", que depende de que el GET de config se aplique y se
    // renderiza fuera de ese gating.
    await screen.findByText(/próxima revisión/i);

    fireEvent.click(
      screen.getByRole("button", { name: /desactivar revisión/i }),
    );

    // El error debe ser visible aunque el bloque de campos esté oculto.
    expect(
      await screen.findByText(/expresión cron inválida/i),
    ).toBeInTheDocument();
    expect(putCalls(calls)).toHaveLength(0);
  });
});

/**
 * `expose-check-interval-ui` — el intervalo de comprobación (`check_interval_ms`)
 * se edita en **segundos** en el formulario, pero viaja en **milisegundos**:
 * al cargar se divide entre 1000 y al guardar se multiplica por 1000. La
 * etiqueta del control refleja el default de `GET /api/update-check/defaults`
 * (también en segundos), sin literales incrustados.
 *
 * El mock usa defaults (`2000 ms` → `2 s`) **distintos** de la config cargada
 * (`3000 ms` → `3 s`) para que la aserción de la etiqueta sea probatoria: si el
 * frontend hardcodeara el valor, la etiqueta no mostraría el `2 s` de la API.
 */
describe("ConfigPage — edición del intervalo de comprobación (segundos)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  function renderCheckInterval(): FetchCall[] {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 480,
              // Distinto del default de la API (2000) para que la etiqueta sea
              // probatoria: el valor precargado es 3 s, la etiqueta dice 2 s.
              check_interval_ms: 3000,
            }),
          ),
        );
      }
      if (url === "/api/update-check/config" && method === "PUT") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              ...body,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "15 3 * * *",
              pull_timeout_secs: 900,
              check_interval_ms: 2000,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 10,
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
    return calls;
  }

  it("carga el intervalo desde la config y lo muestra en segundos (2000 ms → 2)", async () => {
    const calls: FetchCall[] = [];
    const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
      const method = opts?.method ?? "GET";
      const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
      calls.push({ url, method, body });

      if (url === "/api/update-check/config" && method === "GET") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "*/30 * * * *",
              enabled: true,
              notify: false,
              last_run_at: null,
              next_run_at: null,
              pull_timeout_secs: 480,
              check_interval_ms: 2000,
            }),
          ),
        );
      }
      if (url === "/api/update-check/defaults") {
        return Promise.resolve(
          new Response(
            JSON.stringify({
              cron: "15 3 * * *",
              pull_timeout_secs: 900,
              check_interval_ms: 2000,
              update_max_start_attempts: 3,
              update_start_attempt_interval_secs: 10,
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

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    const input = await screen.findByLabelText("Intervalo de comprobación (s)");
    // 2000 ms deben mostrarse como 2 segundos.
    await waitFor(() => expect(input).toHaveValue("2"));
  });

  it("poner 5 segundos y guardar envía check_interval_ms = 5000", async () => {
    const calls = renderCheckInterval();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    const input = await screen.findByLabelText("Intervalo de comprobación (s)");
    await waitFor(() => expect(input).toHaveValue("3"));

    fireEvent.change(input, { target: { value: "5" } });
    expect(input).toHaveValue("5");

    fireEvent.click(
      screen.getByRole("button", { name: /guardar revisión/i }),
    );

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-check/config" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({ check_interval_ms: 5000 });
    });
  });

  it("la etiqueta del intervalo refleja el default de la API (2000 ms → 2 s), no el de la config (3 s)", async () => {
    renderCheckInterval();

    fireEvent.click(screen.getByText("⬆️ Actualizaciones"));

    await screen.findByLabelText("Intervalo de comprobación (s)");

    await waitFor(() =>
      expect(screen.getByText(/Default: 2 s/)).toBeInTheDocument(),
    );
    expect(screen.queryByText(/Default: 3 s/)).not.toBeInTheDocument();
  });
});
