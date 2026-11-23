import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import UpdatesTab from "./UpdatesTab";
import type { UseUpdateCheckConfigResult } from "../useUpdateCheckConfig";
import type { UseDefaultPolicyResult } from "../useDefaultPolicy";

/**
 * RED (ui-pagination-and-vertical-tabs) — Actualizaciones con pestañas
 * verticales.
 *
 * Contrato que debe cumplir la implementación (aún NO implementada):
 *  - `UpdatesTab` renderiza un antd `Tabs` con `tabPosition="left"`.
 *  - Las pestañas son Revisión y Política (Revisión activa por defecto).
 *  - Pulsar Política muestra su contenido y oculta el de Revisión.
 *
 * En RED falla: la implementación actual usa un `Space direction="vertical"`
 * con las dos Cards apiladas (sin `Tabs`).
 *
 * Las Cards hijas se mockean para aislar el comportamiento de las pestañas.
 */
vi.mock("./UpdateCheckCard", () => ({
  default: () => <div>check-tab-body</div>,
}));
vi.mock("./DefaultPolicyCard", () => ({
  default: () => <div>policy-tab-body</div>,
}));

function makeUpdateCheck(): UseUpdateCheckConfigResult {
  return {
    checkConfig: null,
    setCheckConfig: vi.fn(),
    defaults: null,
    setDefaults: vi.fn(),
    ucCron: "",
    setUcCron: vi.fn(),
    ucEnabled: false,
    setUcEnabled: vi.fn(),
    ucNotify: false,
    setUcNotify: vi.fn(),
    pullTimeout: 0,
    setPullTimeout: vi.fn(),
    checkIntervalSecs: 0,
    setCheckIntervalSecs: vi.fn(),
    ucMaxStartAttempts: 0,
    setUcMaxStartAttempts: vi.fn(),
    ucStartAttemptInterval: 0,
    setUcStartAttemptInterval: vi.fn(),
    ucHealthWindow: 0,
    setUcHealthWindow: vi.fn(),
    cronError: null,
    setCronError: vi.fn(),
  };
}

function makeDefaultPolicy(): UseDefaultPolicyResult {
  return {
    defAction: "pull-restart",
    setDefAction: vi.fn(),
    defCleanup: false,
    setDefCleanup: vi.fn(),
    defRollback: false,
    setDefRollback: vi.fn(),
    defMaxStartAttempts: null,
    setDefMaxStartAttempts: vi.fn(),
    defHealthWindow: null,
    setDefHealthWindow: vi.fn(),
  };
}

function renderTab() {
  return render(
    <ConfigProvider>
      <UpdatesTab
        configProp={null}
        updateCheck={makeUpdateCheck()}
        defaultPolicy={makeDefaultPolicy()}
        saving={null}
        setSaving={vi.fn()}
        setError={vi.fn()}
        showSuccess={vi.fn()}
      />
    </ConfigProvider>,
  );
}

describe("UpdatesTab — pestañas verticales", () => {
  it("renderiza Tabs verticales (tabPosition=left) con Revisión y Política", () => {
    const { container } = renderTab();

    expect(container.querySelector(".ant-tabs-left")).not.toBeNull();
    expect(screen.getByRole("tab", { name: "Revisión" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Política" })).toBeInTheDocument();
  });

  it("la pestaña activa por defecto es Revisión", () => {
    renderTab();

    expect(screen.getByRole("tab", { name: "Revisión" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("check-tab-body")).toBeInTheDocument();
  });

  it("pulsar Política muestra su contenido y oculta el de Revisión", () => {
    renderTab();

    fireEvent.click(screen.getByRole("tab", { name: /política/i }));

    expect(screen.getByRole("tab", { name: /política/i })).toHaveAttribute(
      "aria-selected",
      "true",
    );

    // El panel activo (Política) está visible y su contenido presente en el DOM.
    expect(screen.getByRole("tabpanel", { name: /política/i })).toHaveAttribute(
      "aria-hidden",
      "false",
    );
    expect(screen.getByText("policy-tab-body")).toBeInTheDocument();

    // El panel inactivo (Revisión) queda oculto. antd v6 lo mantiene en el
    // DOM como `role="tabpanel"` con `aria-hidden="true"` (ya no usa la clase
    // `.ant-tabs-tabpane` de v5), por lo que se localiza por su contenido.
    const revisionPanel = screen
      .getByText("check-tab-body")
      .closest('[role="tabpanel"]');
    expect(revisionPanel).not.toBeNull();
    expect(revisionPanel).toHaveAttribute("aria-hidden", "true");
  });
});
