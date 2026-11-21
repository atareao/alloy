import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import ContainerActions from "./ContainerActions";
import { makeContainer, makeIdleProgress } from "./testFixtures";
import type { ContainerInfo } from "../types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface SetupOptions {
  container?: ContainerInfo;
  containers?: ContainerInfo[];
  batchPhase?: string;
}

function setup({
  container = makeContainer(),
  containers = [container],
  batchPhase = "idle",
}: SetupOptions = {}) {
  const handlers = {
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
      <ContainerActions
        container={container}
        isMobile={false}
        progress={makeIdleProgress()}
        batchPhase={batchPhase}
        loadingActions={{}}
        containers={containers}
        getPolicy={() => undefined}
        setPolicies={vi.fn()}
        showToast={vi.fn()}
        {...handlers}
      />
    </Wrapper>,
  );

  return { container, ...handlers };
}

describe("ContainerActions", () => {
  it("invoca onInspect con el nombre del contenedor", () => {
    const { onInspect } = setup();
    fireEvent.click(screen.getByRole("button", { name: /inspeccionar/i }));
    expect(onInspect).toHaveBeenCalledWith("web");
  });

  it("invoca onRestart con el nombre del contenedor", () => {
    const { onRestart } = setup();
    fireEvent.click(screen.getByRole("button", { name: /^reload reiniciar$/i }));
    expect(onRestart).toHaveBeenCalledWith("web");
  });

  it("invoca onLogs con el nombre del contenedor", () => {
    const { onLogs } = setup();
    fireEvent.click(screen.getByRole("button", { name: /logs/i }));
    expect(onLogs).toHaveBeenCalledWith("web");
  });

  it("invoca onRemove con el nombre del contenedor", () => {
    const { onRemove } = setup();
    fireEvent.click(screen.getByRole("button", { name: /eliminar/i }));
    expect(onRemove).toHaveBeenCalledWith("web");
  });

  it("muestra Parar y llama onStop cuando el contenedor está running", () => {
    const { onStop } = setup();
    expect(screen.queryByText("Iniciar")).toBeNull();
    fireEvent.click(screen.getByText("Parar"));
    expect(onStop).toHaveBeenCalledWith("web");
  });

  it("muestra Iniciar y llama onStart cuando el contenedor está exited", () => {
    const { onStart } = setup({
      container: makeContainer({ state: "exited", status: "Exited (0) 2 hours ago" }),
    });
    expect(screen.queryByText("Parar")).toBeNull();
    fireEvent.click(screen.getByText("Iniciar"));
    expect(onStart).toHaveBeenCalledWith("web");
  });

  it("deshabilita los botones de acción en modo batch", () => {
    setup({ batchPhase: "active" });

    expect(screen.getByRole("button", { name: /reiniciar/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /parar/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /logs/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /eliminar/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /configurar/i })).toBeDisabled();
  });

  it("muestra acciones de stack con varios contenedores del mismo proyecto", () => {
    const web = makeContainer({ id: "id-web", name: "web", compose_project: "stack-a" });
    const db = makeContainer({
      id: "id-db",
      name: "db",
      compose_project: "stack-a",
      state: "exited",
      status: "Exited (0) 5 minutes ago",
    });

    const { onStackAction } = setup({ container: web, containers: [web, db] });

    fireEvent.click(screen.getByRole("button", { name: /parar todos/i }));
    expect(onStackAction).toHaveBeenCalledWith(
      "stack-a",
      [web, db],
      "stop",
      "Parando todos...",
    );

    fireEvent.click(screen.getByRole("button", { name: /reiniciar todos/i }));
    expect(onStackAction).toHaveBeenCalledWith(
      "stack-a",
      [web, db],
      "restart",
      "Reiniciando todos...",
    );
  });

  it("no muestra acciones de stack con un único contenedor del proyecto", () => {
    setup({ container: makeContainer({ compose_project: "stack-a" }) });
    expect(screen.queryByRole("button", { name: /parar todos/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /reiniciar todos/i })).toBeNull();
  });
});
