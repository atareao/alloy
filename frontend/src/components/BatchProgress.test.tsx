import { describe, it, expect, vi, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { ConfigProvider } from "antd";

// Polyfill ResizeObserver for jsdom (required by antd Card)
beforeAll(() => {
  if (typeof ResizeObserver === "undefined") {
    class ResizeObserverMock {
      observe() {}
      unobserve() {}
      disconnect() {}
    }
    (globalThis as any).ResizeObserver = ResizeObserverMock;
  }
});

import BatchProgress from "./BatchProgress";
import type { UpdateProgress } from "../types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

function makeProgress(
  name: string,
  overrides: Partial<UpdateProgress> = {},
): UpdateProgress {
  return {
    container: name,
    status: "✅ Sin cambios",
    done: true,
    error: null,
    total: 0,
    checked: 0,
    updated: 0,
    errors: 0,
    ...overrides,
  };
}

describe("BatchProgress", () => {
  it("renders null when phase is idle", () => {
    const { container } = render(
      <Wrapper>
        <BatchProgress
          phase="idle"
          batchProgress={{ current: 0, total: 0 }}
          progress={new Map()}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );
    expect(container.innerHTML).toBe("");
  });

  it("does NOT render a scrollable list of entries", () => {
    const progress = new Map<string, UpdateProgress>();
    progress.set("nginx", makeProgress("nginx", { status: "🔍 Verificando nginx:latest...", done: false }));

    const { container } = render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 0, total: 10 }}
          progress={progress}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    // Verify no entry list with scroll exists
    const hasEntryList = Array.from(container.querySelectorAll("*")).some(
      (el) => el.textContent?.includes("nginx") && el.closest('[style*="overflow"]'),
    );
    expect(hasEntryList).toBe(false);
  });

  it("shows current/total — status with the first non-done container", () => {
    const progress = new Map<string, UpdateProgress>();
    progress.set("nginx", makeProgress("nginx", { status: "✅ Sin cambios", done: true }));
    progress.set("crowdsec", makeProgress("crowdsec", { status: "🔍 Verificando crowdsec:latest...", done: false }));
    progress.set("zennotes", makeProgress("zennotes", { status: "⏹️ No aplicable", done: true }));

    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 2, total: 10 }}
          progress={progress}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    // Should show "2 / 10 — 🔍 Verificando crowdsec:latest..."
    expect(screen.getByText(/2 \/ 10/)).toBeInTheDocument();
    expect(screen.getByText(/Verificando crowdsec/)).toBeInTheDocument();
  });

  it('shows "✅ Completado" when all containers are done', () => {
    const progress = new Map<string, UpdateProgress>();
    progress.set("nginx", makeProgress("nginx", { status: "✅ Sin cambios", done: true }));
    progress.set("crowdsec", makeProgress("crowdsec", { status: "✅ actualizado + reiniciado", done: true }));

    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 2, total: 2 }}
          progress={progress}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    expect(screen.getByText(/2 \/ 2/)).toBeInTheDocument();
    expect(screen.getByText(/✅ Completado/)).toBeInTheDocument();
  });

  it("shows live summary with counters from backend", () => {
    const progress = new Map<string, UpdateProgress>();
    // Last entry has the authoritative counters from backend
    progress.set("traefik", makeProgress("traefik", { status: "🔍 Verificando traefik:latest...", done: false, total: 4, checked: 3, updated: 1, errors: 1 }));

    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 3, total: 4 }}
          progress={progress}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    // Total: 4 containers
    expect(screen.getByText(/Total:/)).toBeInTheDocument();
    expect(screen.getByText(/4 containers/)).toBeInTheDocument();
    // 3 revisados (from backend counter)
    expect(screen.getByText(/3 revisados/)).toBeInTheDocument();
    // 1 actualizado (from backend counter)
    expect(screen.getByText(/1 actualizados/)).toBeInTheDocument();
    // 1 error (from backend counter)
    expect(screen.getByText(/1 errores/)).toBeInTheDocument();
    // 1 pendiente (total - checked = 4 - 3)
    expect(screen.getByText(/1 pendientes/)).toBeInTheDocument();
  });

  it("shows Cancel button", () => {
    const onCancel = vi.fn();
    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 0, total: 10 }}
          progress={new Map()}
          onCancel={onCancel}
        />
      </Wrapper>,
    );

    expect(screen.getByRole("button", { name: /cancelar/i })).toBeInTheDocument();
  });

  it("shows the title 'Revisando y actualizando containers...'", () => {
    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 0, total: 10 }}
          progress={new Map()}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    expect(screen.getByText(/Revisando y actualizando containers/)).toBeInTheDocument();
  });

  it("does NOT show 'iniciando...' when no progress entries exist", () => {
    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 0, total: 10 }}
          progress={new Map()}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    // Should NOT contain "iniciando"
    expect(screen.queryByText(/iniciando/)).not.toBeInTheDocument();
    // Should show "🔍 Verificando..." as fallback
    expect(screen.getByText(/Verificando/)).toBeInTheDocument();
  });

  it("shows '🔍 Verificando...' as fallback when no entries in active phase", () => {
    render(
      <Wrapper>
        <BatchProgress
          phase="active"
          batchProgress={{ current: 0, total: 10 }}
          progress={new Map()}
          onCancel={vi.fn()}
        />
      </Wrapper>,
    );

    // Should show "0 / 10 — 🔍 Verificando..."
    expect(screen.getByText(/0 \/ 10/)).toBeInTheDocument();
    expect(screen.getByText(/🔍 Verificando/)).toBeInTheDocument();
  });
});