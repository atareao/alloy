import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import NotificationsTab from "./NotificationsTab";

/**
 * RED (ui-pagination-and-vertical-tabs) — Notificaciones con pestañas
 * verticales.
 *
 * Contrato que debe cumplir la implementación (aún NO implementada):
 *  - `NotificationsTab` renderiza un antd `Tabs` con `tabPosition="left"`.
 *  - Las pestañas son Telegram, Matrix y Valet (Telegram activa por defecto).
 *  - Pulsar Matrix muestra su contenido y oculta el de Telegram.
 *
 * En RED falla: la implementación actual usa un `Space direction="vertical"`
 * con las tres Cards apiladas (sin `Tabs`).
 *
 * Las Cards hijas se mockean para aislar el comportamiento de las pestañas
 * (no aportan nada al contrato y evitan depender de su DOM interno).
 */
vi.mock("./TelegramCard", () => ({
  default: () => <div>telegram-tab-body</div>,
}));
vi.mock("./MatrixCard", () => ({
  default: () => <div>matrix-tab-body</div>,
}));
vi.mock("./ValetCard", () => ({
  default: () => <div>valet-tab-body</div>,
}));

function renderTab() {
  return render(
    <ConfigProvider>
      <NotificationsTab
        configProp={null}
        setConfigProp={vi.fn()}
        saving={null}
        setSaving={vi.fn()}
        testing={null}
        setTesting={vi.fn()}
        setError={vi.fn()}
        showSuccess={vi.fn()}
      />
    </ConfigProvider>,
  );
}

describe("NotificationsTab — pestañas verticales", () => {
  it("renderiza Tabs verticales (tabPosition=left) con Telegram, Matrix y Valet", () => {
    const { container } = renderTab();

    expect(container.querySelector(".ant-tabs-left")).not.toBeNull();
    expect(screen.getByRole("tab", { name: "Telegram" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Matrix" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Valet" })).toBeInTheDocument();
  });

  it("la pestaña activa por defecto es Telegram", () => {
    renderTab();

    expect(screen.getByRole("tab", { name: "Telegram" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("telegram-tab-body")).toBeInTheDocument();
  });

  it("pulsar Matrix muestra su contenido y oculta el de Telegram", () => {
    renderTab();

    fireEvent.click(screen.getByRole("tab", { name: /matrix/i }));

    expect(screen.getByRole("tab", { name: /matrix/i })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    // El panel activo (Matrix) está visible y su contenido presente en el DOM.
    expect(screen.getByRole("tabpanel", { name: /matrix/i })).toHaveAttribute(
      "aria-hidden",
      "false",
    );
    expect(screen.getByText("matrix-tab-body")).toBeInTheDocument();

    // El panel inactivo (Telegram) queda oculto. antd v6 lo mantiene en el
    // DOM como `role="tabpanel"` con `aria-hidden="true"` (ya no usa la clase
    // `.ant-tabs-tabpane` de v5), por lo que se localiza por su contenido.
    const telegramPanel = screen
      .getByText("telegram-tab-body")
      .closest('[role="tabpanel"]');
    expect(telegramPanel).not.toBeNull();
    expect(telegramPanel).toHaveAttribute("aria-hidden", "true");
  });
});
