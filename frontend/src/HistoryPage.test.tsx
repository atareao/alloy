import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import HistoryPage from "./HistoryPage";
import type { HistoryEntry } from "./types";

/**
 * RED (task 1.13) — guarda de arranque (update-rollback-guard).
 *
 * Contrato que deben cumplir estos tests (aún NO implementado):
 *  - `status === "rolled-back"`  → etiqueta propia, color warning (amarillo) y tooltip propio.
 *  - `status === "failed-stopped"` → etiqueta propia, color error (rojo) y tooltip propio.
 *  - `attempts` (nº de intentos de arranque) se muestra en una columna "Intentos".
 *  - `status === "update-check-stack"` se mapea como estado de comprobación (D8).
 *
 * En RED fallan porque `formatStatus`/`statusColor`/`statusTooltipLabel` no conocen los
 * estados nuevos y `HistoryPage` no tiene columna de intentos; además `HistoryEntry`
 * (src/types.ts) todavía no declara `attempts`.
 */

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

function makeEntry(overrides: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    container: "nginx",
    image: "nginx:latest",
    old_digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    new_digest: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    timestamp: "2026-01-02T03:04:05Z",
    status: "updated",
    duration_ms: 1500,
    attempts: 0,
    ...overrides,
  };
}

function renderHistory(entries: HistoryEntry[]) {
  render(
    <Wrapper>
      <HistoryPage history={entries} setHistory={() => {}} />
    </Wrapper>,
  );
}

function tagFor(label: string | RegExp): HTMLElement {
  const el = screen.getByText(label);
  const tag = el.closest(".ant-tag");
  if (!tag) throw new Error(`No se encontró un Tag para ${String(label)}`);
  return tag as HTMLElement;
}

/**
 * Fuerza el resultado de `useMediaQuery("(max-width: 768px)")`, que decide entre
 * la tabla (escritorio) y las tarjetas (móvil). El `test-setup.ts` global fija
 * `matches: false`; aquí se restaura tras cada test.
 */
function setMobileViewport(mobile: boolean) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string) => ({
      matches: mobile && query === "(max-width: 768px)",
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

describe("HistoryPage — estados de la guarda (rolled-back / failed-stopped)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    // HistoryPage solo llama a la API al limpiar historial; se mockea igualmente.
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("ok")));
  });

  it("rolled-back: etiqueta propia, color warning y tooltip explicativo", async () => {
    renderHistory([makeEntry({ status: "rolled-back" })]);

    const tag = tagFor("↩️ Rollback aplicado");
    expect(tag).toHaveClass("ant-tag-warning");

    fireEvent.mouseEnter(tag);
    await waitFor(() => {
      expect(document.body.textContent ?? "").toMatch(
        /se restauró la imagen anterior/i,
      );
    });
  });

  it("failed-stopped: etiqueta propia, color error y tooltip explicativo", async () => {
    renderHistory([makeEntry({ status: "failed-stopped" })]);

    const tag = tagFor("🛑 Fallo terminal");
    expect(tag).toHaveClass("ant-tag-error");

    fireEvent.mouseEnter(tag);
    await waitFor(() => {
      expect(document.body.textContent ?? "").toMatch(
        /el contenedor quedó parado/i,
      );
    });
  });

  it("muestra el número de intentos de arranque en una columna propia", () => {
    renderHistory([
      makeEntry({ status: "rolled-back", attempts: 7, duration_ms: 1000 }),
    ]);

    expect(screen.getByText("Intentos")).toBeInTheDocument();
    expect(screen.getByText("7")).toBeInTheDocument();
  });

  it("mapea update-check-stack como estado de comprobación (D8)", () => {
    renderHistory([makeEntry({ status: "update-check-stack" })]);

    expect(screen.getByText("📦 update-check")).toBeInTheDocument();
  });
});

describe("HistoryPage — tarjeta móvil (fondo coherente con el nivel del estado)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("ok")));
    setMobileViewport(true);
  });

  afterEach(() => {
    setMobileViewport(false);
  });

  it("rolled-back: tarjeta con fondo de aviso (warning), no de error, y etiqueta propia", () => {
    renderHistory([makeEntry({ status: "rolled-back" })]);

    // En móvil se renderiza la tarjeta, no la tabla.
    expect(screen.queryByRole("table")).not.toBeInTheDocument();

    const tag = tagFor("↩️ Rollback aplicado");
    expect(tag).toHaveClass("ant-tag-warning");

    const card = tag.closest(".ant-card") as HTMLElement | null;
    expect(card).not.toBeNull();
    const style = card!.getAttribute("style") ?? "";
    expect(style).toContain("--ant-color-warning-bg");
    expect(style).not.toContain("--ant-color-error-bg");
  });

  it("failed-stopped: tarjeta con fondo de error y etiqueta de error", () => {
    renderHistory([makeEntry({ status: "failed-stopped" })]);

    const tag = tagFor("🛑 Fallo terminal");
    expect(tag).toHaveClass("ant-tag-error");

    const card = tag.closest(".ant-card") as HTMLElement | null;
    const style = card?.getAttribute("style") ?? "";
    expect(style).toContain("--ant-color-error-bg");
  });
});
