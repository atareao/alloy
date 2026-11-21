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

function defaultContainers(): ContainerInfo[] {
  return [
    makeContainer({ id: "id-web", name: "web", has_update: true }),
    makeContainer({
      id: "id-db",
      name: "db",
      state: "exited",
      status: "Exited (0) 2 hours ago",
    }),
  ];
}

interface SetupOptions {
  containers?: ContainerInfo[];
  batchPhase?: "idle" | "active";
  searchQuery?: string;
  debouncedSearch?: string;
  stateFilter?: string[];
  showPendingUpdates?: boolean;
  page?: number;
  pageSize?: number;
  total?: number;
}

function makeHandlers() {
  return {
    setShowSummary: vi.fn(),
    onCheckAll: vi.fn(),
    setSearchQuery: vi.fn(),
    setStateFilter: vi.fn(),
    setShowPendingUpdates: vi.fn(),
    onPageChange: vi.fn(),
    onPageSizeChange: vi.fn(),
  };
}

type Handlers = ReturnType<typeof makeHandlers>;

function dashboardElement(
  overrides: SetupOptions,
  handlers: Handlers,
): React.ReactElement {
  const containers = overrides.containers ?? defaultContainers();
  return (
    <Wrapper>
      <DashboardPage
        containers={containers}
        progress={{
          total: 0,
          checked: 0,
          updated: 0,
          errors: 0,
          checking: "",
        }}
        containersLoaded
        batchPhase={overrides.batchPhase ?? "idle"}
        checkResults={emptyResults}
        updateResults={emptyResults}
        showSummary={false}
        checkConfig={null}
        summary={{
          total: containers.length,
          running: containers.filter((c) => c.state === "running").length,
          stopped: containers.filter((c) => c.state !== "running").length,
          paused: 0,
          with_updates: containers.filter((c) => c.has_update).length,
        }}
        highlights={{}}
        searchQuery={overrides.searchQuery ?? ""}
        debouncedSearch={overrides.debouncedSearch ?? ""}
        stateFilter={overrides.stateFilter ?? []}
        showPendingUpdates={overrides.showPendingUpdates ?? false}
        total={overrides.total ?? containers.length}
        page={overrides.page ?? 1}
        pageSize={overrides.pageSize ?? 20}
        {...handlers}
      />
    </Wrapper>
  );
}

function setup(overrides: SetupOptions = {}) {
  const handlers = makeHandlers();
  const view = render(dashboardElement(overrides, handlers));
  return { ...handlers, ...view };
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
    const { setSearchQuery } = setup();
    fireEvent.change(
      screen.getByPlaceholderText("Buscar por nombre, imagen, stack..."),
      { target: { value: "web" } },
    );
    // Filtering now happens on the server: DashboardPage only propagates the query.
    expect(setSearchQuery).toHaveBeenCalledWith("web");
  });

  it("filtra por estado", () => {
    const { setStateFilter } = setup();
    fireEvent.click(screen.getByText("running"));
    expect(setStateFilter).toHaveBeenCalledWith(["running"]);
  });

  it("filtra por actualizaciones pendientes", () => {
    const { setShowPendingUpdates } = setup();
    fireEvent.click(screen.getByRole("switch"));
    expect(setShowPendingUpdates).toHaveBeenCalledWith(true);
  });

  it("colapsa las filas expandidas al cambiar de página", async () => {
    const handlers = makeHandlers();
    const { rerender } = render(dashboardElement({ page: 1 }, handlers));

    expandRow("web");
    expect(screen.getByText("Parar")).toBeInTheDocument();

    rerender(dashboardElement({ page: 2 }, handlers));

    await waitFor(() =>
      expect(screen.queryByText("Parar")).not.toBeInTheDocument(),
    );
  });

  it("no colapsa al cambiar searchQuery sin debounce", () => {
    const handlers = makeHandlers();
    const { rerender } = render(dashboardElement({}, handlers));

    expandRow("web");
    expect(screen.getByText("Parar")).toBeInTheDocument();

    // Typing updates searchQuery immediately, but the search is not applied
    // until it is debounced — rows must stay expanded.
    rerender(dashboardElement({ searchQuery: "web" }, handlers));

    expect(screen.getByText("Parar")).toBeInTheDocument();
  });

  it("colapsa al aplicarse la búsqueda debounced", async () => {
    const handlers = makeHandlers();
    const { rerender } = render(dashboardElement({}, handlers));

    expandRow("web");
    expect(screen.getByText("Parar")).toBeInTheDocument();

    rerender(
      dashboardElement(
        { searchQuery: "web", debouncedSearch: "web" },
        handlers,
      ),
    );

    await waitFor(() =>
      expect(screen.queryByText("Parar")).not.toBeInTheDocument(),
    );
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
