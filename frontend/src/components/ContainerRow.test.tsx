import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import ContainerRow from "./ContainerRow";
import { makeContainer, makeIdleProgress } from "./testFixtures";
import type { BatchProgress, ContainerInfo } from "../types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface SetupOptions {
  container?: ContainerInfo;
  expanded?: boolean;
  isMobile?: boolean;
  progress?: BatchProgress;
}

function setup({
  container = makeContainer(),
  expanded = false,
  isMobile = false,
  progress = makeIdleProgress(),
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
        progress={progress}
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

  it("muestra el nombre completo sin truncado manual en la variante móvil", () => {
    setup({
      container: makeContainer({ name: "very-long-container-name" }),
      isMobile: true,
    });
    expect(screen.getByText("very-long-container-name")).toBeInTheDocument();
    expect(screen.queryByText(/\.\.\./)).toBeNull();
  });

  it("muestra el nombre completo sin truncado manual en la variante de escritorio", () => {
    setup({
      container: makeContainer({ name: "very-long-container-name" }),
      isMobile: false,
    });
    expect(screen.getByText("very-long-container-name")).toBeInTheDocument();
    expect(screen.queryByText(/\.\.\./)).toBeNull();
  });

  it("mantiene el nombre y el estado completos en el nodo tipográfico (la elipsis es CSS, no manual)", () => {
    const name = "very-long-container-name";
    const status = "Up 3 hours (unhealthy) - a very long status text";
    setup({ container: makeContainer({ name, status }), isMobile: true });

    // En jsdom no hay layout: `isEllipsis` es false y el Tooltip de antd queda
    // `disabled`, así que no es verificable. En su lugar comprobamos que el
    // texto COMPLETO sigue en el DOM (sin recorte manual) y que el nodo lleva
    // la elipsis CSS de antd.
    const nameNode = screen.getByText(name).closest(".ant-typography");
    const statusNode = screen.getByText(status).closest(".ant-typography");
    expect(nameNode).toHaveTextContent(name);
    expect(statusNode).toHaveTextContent(status);
    expect(nameNode).toHaveClass("ant-typography-ellipsis");
    expect(statusNode).toHaveClass("ant-typography-ellipsis");
  });

  it("hace encogible el nombre (minWidth: 0) para que la elipsis sea efectiva", () => {
    setup({
      container: makeContainer({ name: "very-long-container-name" }),
      isMobile: true,
    });
    const nameEl = screen
      .getByText("very-long-container-name")
      .closest(".ant-typography");
    expect(nameEl).not.toBeNull();
    expect(nameEl).toHaveStyle({ minWidth: "0px" });
  });

  it("con un nombre muy largo siguen presentes el badge de actualización y el botón de expansión (regresión de presencia, no de layout)", () => {
    setup({
      container: makeContainer({
        name: "very-long-container-name",
        has_update: true,
      }),
      isMobile: false,
    });
    expect(screen.getByText("!")).toBeInTheDocument();
    expect(screen.getByText("very-long-container-name")).toBeInTheDocument();
    // El botón de expansión es accesible por el aria-label del icono de antd ("down").
    expect(
      screen.getByRole("button", { name: /down/i }),
    ).toBeInTheDocument();
  });

  it("muestra el progreso completo (sin recortar) mientras se comprueba el contenedor", () => {
    const name = "very-long-container-name";
    setup({
      container: makeContainer({ name }),
      progress: {
        total: 1,
        checked: 0,
        updated: 0,
        errors: 0,
        checking: name,
      },
    });
    // `isUpdating` exige `checking === name`, así que la etiqueta de progreso
    // muestra el mismo texto que la cabecera. Localizamos todos los nodos con
    // ese texto (cabecera + progreso) y comprobamos que ambos están completos.
    const nodes = screen.getAllByText(name);
    expect(nodes.length).toBeGreaterThanOrEqual(2);
    for (const node of nodes) {
      expect(node.closest(".ant-typography")).toHaveTextContent(name);
    }
    // No hay recorte manual del progreso (el `slice(0, 20) + "..."` anterior).
    expect(screen.queryByText(/\.\.\./)).toBeNull();
  });
});
