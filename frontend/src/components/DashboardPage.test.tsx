import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  within,
} from "@testing-library/react";
import { ConfigProvider } from "antd";
import DashboardPage from "./DashboardPage";
import { makeContainer, makeInspectData } from "./testFixtures";
import { EventSourceMock } from "../test-setup";
import type { BatchResults } from "./BatchProgress";
import type { ContainerInfo } from "../types";

const { apiFetchMock } = vi.hoisted(() => ({ apiFetchMock: vi.fn() }));

vi.mock("../api", () => ({ apiFetch: apiFetchMock }));

function Wrapper({ children }: { children: React.ReactNode }) {
  return (
    <ConfigProvider theme={{ token: { motion: false } }}>
      {children}
    </ConfigProvider>
  );
}

const emptyResults: BatchResults = {
  total: 0,
  updated: 0,
  uptodate: 0,
  failed: 0,
  done: 0,
  errors: [],
};

interface SetupOptions {
  containers?: ContainerInfo[];
  batchPhase?: "idle" | "active";
}

function setup({
  containers = [
    makeContainer({ id: "id-web", name: "web", has_update: true }),
    makeContainer({
      id: "id-db",
      name: "db",
      state: "exited",
      status: "Exited (0) 2 hours ago",
    }),
  ],
  batchPhase = "idle",
}: SetupOptions = {}) {
  render(
    <Wrapper>
      <DashboardPage
        containers={containers}
        setContainers={vi.fn()}
        progress={{
          total: 0,
          checked: 0,
          updated: 0,
          errors: 0,
          checking: "",
        }}
        containersLoaded
        batchPhase={batchPhase}
        checkResults={emptyResults}
        updateResults={emptyResults}
        showSummary={false}
        setShowSummary={vi.fn()}
        checkConfig={null}
        onCheckAll={vi.fn()}
        summary={{
          total: containers.length,
          running: containers.filter((c) => c.state === "running").length,
          stopped: containers.filter((c) => c.state !== "running").length,
          paused: 0,
          with_updates: containers.filter((c) => c.has_update).length,
        }}
        highlights={{}}
      />
    </Wrapper>,
  );
}

function expandRow(name: string) {
  fireEvent.click(screen.getByText(name));
}

describe("DashboardPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    EventSourceMock.reset();
    apiFetchMock.mockImplementation((path: string) => {
      if (path === "/api/update-policies") {
        return Promise.resolve(new Response(JSON.stringify([])));
      }
      if (path.endsWith("/inspect")) {
        return Promise.resolve(new Response(JSON.stringify(makeInspectData())));
      }
      return Promise.resolve(new Response("ok", { status: 200 }));
    });
  });

  it("renderiza la tabla con los contenedores", async () => {
    setup();
    expect(screen.getByText("web")).toBeInTheDocument();
    expect(screen.getByText("db")).toBeInTheDocument();
    expect(screen.getByText("Sin stack")).toBeInTheDocument();
    await waitFor(() =>
      expect(apiFetchMock).toHaveBeenCalledWith("/api/update-policies"),
    );
  });

  it("filtra por búsqueda", () => {
    setup();
    fireEvent.change(
      screen.getByPlaceholderText("Buscar por nombre, imagen, stack..."),
      { target: { value: "web" } },
    );
    expect(screen.getByText("web")).toBeInTheDocument();
    expect(screen.queryByText("db")).toBeNull();
  });

  it("filtra por estado", () => {
    setup();
    fireEvent.click(screen.getByText("running"));
    expect(screen.getByText("web")).toBeInTheDocument();
    expect(screen.queryByText("db")).toBeNull();
  });

  it("filtra por actualizaciones pendientes", () => {
    setup();
    fireEvent.click(screen.getByRole("switch"));
    expect(screen.getByText("web")).toBeInTheDocument();
    expect(screen.queryByText("db")).toBeNull();
  });

  it("reinicia un contenedor llamando a apiFetch con la URL y el método correctos", async () => {
    setup();
    expandRow("web");
    fireEvent.click(screen.getByRole("button", { name: /^reload reiniciar$/i }));

    await waitFor(() =>
      expect(apiFetchMock).toHaveBeenCalledWith(
        "/api/containers/web/restart",
        { method: "POST" },
      ),
    );
  });

  it("inicia un contenedor exited llamando a apiFetch con la URL y el método correctos", async () => {
    setup();
    expandRow("db");
    fireEvent.click(screen.getByText("Iniciar"));

    await waitFor(() =>
      expect(apiFetchMock).toHaveBeenCalledWith("/api/containers/db/start", {
        method: "POST",
      }),
    );
  });

  it("para un contenedor running llamando a apiFetch con la URL y el método correctos", async () => {
    setup();
    expandRow("web");
    fireEvent.click(screen.getByText("Parar"));

    await waitFor(() =>
      expect(apiFetchMock).toHaveBeenCalledWith("/api/containers/web/stop", {
        method: "POST",
      }),
    );
  });

  it("abre los logs y crea el EventSource con la URL correcta", async () => {
    setup();
    expandRow("web");
    fireEvent.click(screen.getByRole("button", { name: /logs/i }));

    await waitFor(() =>
      expect(EventSourceMock.last()?.url).toBe("/api/containers/web/logs"),
    );
  });

  it("elimina un contenedor tras confirmar y cierra el diálogo", async () => {
    setup();
    expandRow("web");
    fireEvent.click(screen.getByRole("button", { name: /eliminar/i }));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /eliminar/i }));

    await waitFor(() =>
      expect(apiFetchMock).toHaveBeenCalledWith("/api/containers/web/remove", {
        method: "POST",
      }),
    );
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
  });

  it("abre y cierra el modal de inspección", async () => {
    setup();
    expandRow("web");
    fireEvent.click(screen.getByRole("button", { name: /inspeccionar/i }));

    await waitFor(() =>
      expect(apiFetchMock).toHaveBeenCalledWith("/api/containers/web/inspect"),
    );
    expect(await screen.findByText("abc123")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
  });
});
