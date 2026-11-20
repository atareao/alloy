import { describe, it, expect } from "vitest";
import { mapBatchToResults } from "./batchResults";
import type { BatchProgress } from "./types";

describe("mapBatchToResults", () => {
  it("mapea los contadores reales a check/update", () => {
    const bp: BatchProgress = {
      total: 10,
      checked: 10,
      updated: 2,
      errors: 1,
      checking: "",
      updatable: 3,
      uptodate: 5,
      failed: 1,
    };

    const r = mapBatchToResults(bp);

    // Tarjeta Comprobación
    expect(r.check.total).toBe(10); // Comprobados
    expect(r.check.updated).toBe(3); // Actualizables
    expect(r.check.uptodate).toBe(5); // Actuales
    expect(r.check.failed).toBe(1); // Errores
    expect(r.check.done).toBe(10);

    // Tarjeta Actualización
    expect(r.update.done).toBe(2); // Actualizados
    expect(r.update.failed).toBe(1); // Fallos
  });

  it("tolera payloads legacy sin updatable/uptodate/failed", () => {
    const bp = {
      total: 4,
      checked: 4,
      updated: 0,
      errors: 0,
      checking: "",
    } as BatchProgress;

    const r = mapBatchToResults(bp);

    expect(r.check.total).toBe(4);
    expect(r.check.updated).toBe(0);
    expect(r.check.uptodate).toBe(0);
    expect(r.check.failed).toBe(0);
    expect(r.update.failed).toBe(0);
  });
});