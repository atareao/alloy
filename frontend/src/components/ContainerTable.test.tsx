import { describe, it, expect, vi } from "vitest";
import { act, render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import { useState } from "react";
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
}

function setup({
  sortedGroups = [],
  noStack = [],
  searchQuery = "",
  stateFilter = [],
  showPendingUpdates = false,
  batchPhase = "idle",
}: SetupOptions = {}) {
  const handlers = {
    setSearchQuery: vi.fn(),
    setStateFilter: vi.fn(),
    setShowPendingUpdates: vi.fn(),
    onCheckAll: vi.fn(),
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
});

/**
 * Dashboard sin paginación.
 *
 * Contrato que cumple la implementación:
 *  - `ContainerTable` ya NO renderiza ningún `Pagination` ni selector de tamaño.
 *  - Se eliminaron las props `total`, `page`, `pageSize`, `onPageChange` y
 *    `onPageSizeChange` (el Dashboard pinta la lista filtrada entera).
 */
describe("ContainerTable — Dashboard sin paginación", () => {
  it("muestra todas las filas de un dataset de 37 contenedores y no renderiza Pagination", () => {
    const containers = Array.from({ length: 37 }, (_, i) =>
      makeContainer({ id: `id-c-${i}`, name: `c-${i}` }),
    );
    const renderRow = (c: ContainerInfo) => (
      <div key={c.id} data-testid="row">
        {c.name}
      </div>
    );

    const { container } = render(
      <Wrapper>
        <ContainerTable
          sortedGroups={[]}
          noStack={containers}
          searchQuery=""
          setSearchQuery={vi.fn()}
          stateFilter={[]}
          setStateFilter={vi.fn()}
          showPendingUpdates={false}
          setShowPendingUpdates={vi.fn()}
          availableStates={["running"]}
          isMobile={false}
          onCheckAll={vi.fn()}
          lastCheck={null}
          nextCheck={null}
          expandedStacks={{}}
          renderGroup={vi.fn()}
          renderRow={renderRow}
          batchPhase="idle"
        />
      </Wrapper>,
    );

    // No hay ningún control de paginación en el Dashboard.
    expect(container.querySelector(".ant-pagination")).toBeNull();
    // Se renderizan las 37 filas, sin recorte por página.
    expect(screen.getAllByTestId("row")).toHaveLength(37);
  });
});

// ── Accesibilidad de los filtros (criterio de HistoryPage 3.11) ─────
describe("ContainerTable — filtros accesibles (a11y)", () => {
  function TagTestHarness() {
    const [sf, setSf] = useState<string[]>([]);
    return (
      <Wrapper>
        <ContainerTable
          sortedGroups={[]}
          noStack={[]}
          searchQuery=""
          setSearchQuery={() => {}}
          stateFilter={sf}
          setStateFilter={setSf}
          showPendingUpdates={false}
          setShowPendingUpdates={() => {}}
          availableStates={["running", "exited"]}
          isMobile={false}
          onCheckAll={() => {}}
          lastCheck={null}
          nextCheck={null}
          expandedStacks={{}}
          renderGroup={() => null}
          renderRow={() => null}
          batchPhase="idle"
        />
      </Wrapper>
    );
  }

  it("el tag de estado se activa con teclado (foco + Enter/Space) y expone aria-pressed", async () => {
    render(<TagTestHarness />);
    const chip = screen.getByText("running");
    // Es un botón alcanzable por teclado, no un `span` inerte.
    expect(chip).toHaveAttribute("role", "button");
    expect(chip).toHaveAttribute("tabindex", "0");
    expect(chip).toHaveAttribute("aria-pressed", "false");

    chip.focus();
    expect(chip).toHaveFocus();

    await act(async () => {
      fireEvent.keyDown(chip, { key: "Enter" });
    });
    expect(chip).toHaveAttribute("aria-pressed", "true");

    // `Space` también activa el chip (alternándolo de nuevo).
    await act(async () => {
      fireEvent.keyDown(chip, { key: " " });
    });
    expect(chip).toHaveAttribute("aria-pressed", "false");
  });

  function LabelTestHarness() {
    const [show, setShow] = useState(false);
    return (
      <Wrapper>
        <ContainerTable
          sortedGroups={[]}
          noStack={[]}
          searchQuery=""
          setSearchQuery={() => {}}
          stateFilter={[]}
          setStateFilter={() => {}}
          showPendingUpdates={show}
          setShowPendingUpdates={setShow}
          availableStates={["running"]}
          isMobile={false}
          onCheckAll={() => {}}
          lastCheck={null}
          nextCheck={null}
          expandedStacks={{}}
          renderGroup={() => null}
          renderRow={() => null}
          batchPhase="idle"
        />
      </Wrapper>
    );
  }

  it("la etiqueta 'Solo pendientes de actualizar' está asociada al switch", async () => {
    render(<LabelTestHarness />);
    // Click en la etiqueta activa el control asociado (htmlFor/id).
    await act(async () => {
      fireEvent.click(screen.getByText("Solo pendientes de actualizar"));
    });
    expect(screen.getByRole("switch")).toBeChecked();
  });
});
