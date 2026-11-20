import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import SummaryDialog from "./SummaryDialog";

describe("SummaryDialog", () => {
  it("muestra los contadores reales del resumen de comprobación", () => {
    render(
      <SummaryDialog
        opened
        onClose={vi.fn()}
        phase="idle"
        checkResults={{
          total: 10,
          updated: 3,
          uptodate: 5,
          failed: 2,
          done: 10,
          errors: [],
        }}
        updateResults={{
          total: 10,
          updated: 2,
          uptodate: 0,
          failed: 1,
          done: 2,
          errors: [],
        }}
      />,
    );

    // Tarjeta Comprobación: valores reales (no ceros).
    expect(screen.getByText("Comprobados")).toBeInTheDocument();
    expect(screen.getByText("10")).toBeInTheDocument(); // Comprobados

    expect(screen.getByText("Actualizables")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument(); // Actualizables

    expect(screen.getByText("Actuales")).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument(); // Actuales

    expect(screen.getByText("Errores")).toBeInTheDocument();

    // Tarjeta Actualización (hasUpdates = done > 0): se pinta.
    expect(screen.getByText("Actualizados")).toBeInTheDocument();
    expect(screen.getByText("Fallos")).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument(); // Fallos

    // "2" aparece dos veces: checkResults.failed (Errores) y
    // updateResults.done (Actualizados).
    expect(screen.getAllByText("2")).toHaveLength(2);
  });
});
