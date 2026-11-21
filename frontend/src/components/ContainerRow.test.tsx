import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import ContainerRow from "./ContainerRow";
import { makeContainer, makeIdleProgress } from "./testFixtures";
import type { ContainerInfo } from "../types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface SetupOptions {
  container?: ContainerInfo;
  expanded?: boolean;
  isMobile?: boolean;
}

function setup({
  container = makeContainer(),
  expanded = false,
  isMobile = false,
}: SetupOptions = {}) {
  const handlers = {
    onToggleExpand: vi.fn(),
    onInspect: vi.fn(),
    onLogs: vi.fn(),
    onStart: vi.fn(),
    onStop: vi.fn(),
    onRestart: vi.fn(),
    onRemove: vi.fn(),
    onStackAction: vi.fn(),
  };

  render(
    <Wrapper>
      <ContainerRow
        container={container}
        isMobile={isMobile}
        expanded={expanded}
        progress={makeIdleProgress()}
        getPolicy={() => undefined}
        setPolicies={vi.fn()}
        loadingActions={{}}
        batchPhase="idle"
        containers={[container]}
        {...handlers}
      />
    </Wrapper>,
  );

  return { container, ...handlers };
}

describe("ContainerRow", () => {
  it("muestra el nombre y el estado del contenedor", () => {
    setup();
    expect(screen.getByText("web")).toBeInTheDocument();
    expect(screen.getByText("Up 2 hours")).toBeInTheDocument();
  });

  it("muestra el badge de actualización cuando has_update es true", () => {
    setup({ container: makeContainer({ has_update: true }) });
    expect(screen.getByText("!")).toBeInTheDocument();
  });

  it("no muestra el badge de actualización cuando no hay update", () => {
    setup();
    expect(screen.queryByText("!")).toBeNull();
  });

  it("llama onToggleExpand al pulsar la cabecera de la fila", () => {
    const { onToggleExpand } = setup();
    fireEvent.click(screen.getByText("web"));
    expect(onToggleExpand).toHaveBeenCalledWith("web");
  });

  it("muestra las acciones sólo cuando está expandida", () => {
    setup();
    expect(
      screen.queryByRole("button", { name: /inspeccionar/i }),
    ).toBeNull();
  });

  it("expone las acciones y sus callbacks cuando está expandida", () => {
    const { onInspect, onRestart, onStop } = setup({ expanded: true });

    fireEvent.click(screen.getByRole("button", { name: /inspeccionar/i }));
    expect(onInspect).toHaveBeenCalledWith("web");

    fireEvent.click(screen.getByRole("button", { name: /reiniciar/i }));
    expect(onRestart).toHaveBeenCalledWith("web");

    fireEvent.click(screen.getByRole("button", { name: /parar/i }));
    expect(onStop).toHaveBeenCalledWith("web");
  });

  it("trunca el nombre en la variante móvil", () => {
    setup({
      container: makeContainer({ name: "very-long-container-name" }),
      isMobile: true,
    });
    expect(screen.getByText("very-long...")).toBeInTheDocument();
  });
});
