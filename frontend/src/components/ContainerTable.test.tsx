import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ConfigProvider } from "antd";
import ContainerTable from "./ContainerTable";
import { makeContainer } from "./testFixtures";
import type { ContainerInfo } from "../types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface SetupOptions {
  sortedGroups?: [string, ContainerInfo[]][];
  noStack?: ContainerInfo[];
  searchQuery?: string;
  stateFilter?: string[];
  showPendingUpdates?: boolean;
  batchPhase?: "idle" | "active";
  total?: number;
  page?: number;
  pageSize?: number;
}

function setup({
  sortedGroups = [],
  noStack = [],
  searchQuery = "",
  stateFilter = [],
  showPendingUpdates = false,
  batchPhase = "idle",
  total = 0,
  page = 1,
  pageSize = 20,
}: SetupOptions = {}) {
  const handlers = {
    setSearchQuery: vi.fn(),
    setStateFilter: vi.fn(),
    setShowPendingUpdates: vi.fn(),
    onCheckAll: vi.fn(),
    onPageChange: vi.fn(),
    onPageSizeChange: vi.fn(),
  };
  const renderGroup = vi.fn((project: string, items: ContainerInfo[]) => (
    <div key={project} data-testid={`group-${project}`}>
      {project}:{items.length}
    </div>
  ));
  const renderRow = vi.fn((c: ContainerInfo) => <div key={c.id}>{c.name}</div>);

  render(
    <Wrapper>
      <ContainerTable
        sortedGroups={sortedGroups}
        noStack={noStack}
        searchQuery={searchQuery}
        stateFilter={stateFilter}
        showPendingUpdates={showPendingUpdates}
        availableStates={["running", "exited"]}
        isMobile={false}
        lastCheck={null}
        nextCheck={null}
        expandedStacks={{}}
        renderGroup={renderGroup}
        renderRow={renderRow}
        batchPhase={batchPhase}
        total={total}
        page={page}
        pageSize={pageSize}
        {...handlers}
      />
    </Wrapper>,
  );

  return { ...handlers };
}

describe("ContainerTable", () => {
  it("propaga la búsqueda al cambiar el input", () => {
    const { setSearchQuery } = setup();
    fireEvent.change(
      screen.getByPlaceholderText("Buscar por nombre, imagen, stack..."),
      { target: { value: "web" } },
    );
    expect(setSearchQuery).toHaveBeenCalledWith("web");
  });

  it("añade un filtro de estado al pulsar un tag", () => {
    const { setStateFilter } = setup();
    fireEvent.click(screen.getByText("running"));
    expect(setStateFilter).toHaveBeenCalledWith(["running"]);
  });

  it("quita un filtro de estado ya activo", () => {
    const { setStateFilter } = setup({ stateFilter: ["running"] });
    fireEvent.click(screen.getByText("running"));
    expect(setStateFilter).toHaveBeenCalledWith([]);
  });

  it("llama onCheckAll desde el botón de comprobar", () => {
    const { onCheckAll } = setup();
    fireEvent.click(screen.getByRole("button", { name: /check/i }));
    expect(onCheckAll).toHaveBeenCalledTimes(1);
  });

  it("activa el filtro de pendientes con el switch", () => {
    const { setShowPendingUpdates } = setup();
    fireEvent.click(screen.getByRole("switch"));
    expect(setShowPendingUpdates).toHaveBeenCalledWith(true);
  });

  it("renderiza los grupos de stack y los contenedores sin stack", () => {
    const web = makeContainer({ id: "id-web", name: "web" });
    const db = makeContainer({ id: "id-db", name: "db" });
    const lonely = makeContainer({ id: "id-lonely", name: "lonely" });

    setup({
      sortedGroups: [["stack-a", [web, db]]],
      noStack: [lonely],
    });

    expect(screen.getByTestId("group-stack-a")).toHaveTextContent("stack-a:2");
    expect(screen.getByText("lonely")).toBeInTheDocument();
  });

  it("oculta la barra de búsqueda y filtros en modo batch", () => {
    setup({ batchPhase: "active" });
    expect(
      screen.queryByPlaceholderText("Buscar por nombre, imagen, stack..."),
    ).toBeNull();
    expect(screen.queryByRole("button", { name: /check/i })).toBeNull();
    expect(screen.queryByText("Solo pendientes de actualizar")).toBeNull();
  });

  it("oculta el paginador en modo batch", () => {
    setup({ batchPhase: "active", total: 25, pageSize: 10 });
    expect(document.querySelector(".ant-pagination")).toBeNull();
  });

  it("muestra el total recibido en el paginador", () => {
    setup({ total: 25, pageSize: 10 });
    expect(screen.getByText("25 contenedores")).toBeInTheDocument();
  });

  it("pide cambiar de página (sin tocar el tamaño) al pulsar otra página", () => {
    const { onPageChange, onPageSizeChange } = setup({
      total: 25,
      pageSize: 10,
      page: 1,
    });
    fireEvent.click(screen.getByLabelText("Page 2"));
    expect(onPageChange).toHaveBeenCalledWith(2);
    expect(onPageSizeChange).not.toHaveBeenCalled();
  });

  it("pide cambiar el tamaño (y no la página) al elegir otra opción", async () => {
    const { onPageChange, onPageSizeChange } = setup({
      total: 100,
      pageSize: 10,
      page: 1,
    });

    fireEvent.mouseDown(document.querySelector(".ant-select")!);
    const option = await screen.findByTitle("50 / page");
    fireEvent.click(option);

    await waitFor(() => expect(onPageSizeChange).toHaveBeenCalledWith(50));
    expect(onPageChange).not.toHaveBeenCalled();
  });
});
