import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ConfigProvider } from "antd";
import PolicyActionButton from "./PolicyActionButton";
import type { UpdatePolicy } from "../types";

/**
 * Auditoría (Hallazgo 2): el editor de política **por contenedor** debía
 * exponer los overrides de la guarda (`max_start_attempts`,
 * `health_window_secs`) y enviarlos SIEMPRE en el PUT para no resetear los
 * existentes.
 */

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface FetchCall {
  url: string;
  method: string;
  body: unknown;
}

function installFetchMock(): FetchCall[] {
  const calls: FetchCall[] = [];
  const mock = vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
    const method = opts?.method ?? "GET";
    const body = opts?.body ? JSON.parse(opts.body as string) : undefined;
    calls.push({ url, method, body });
    return Promise.resolve(new Response(JSON.stringify({ ...body })));
  });
  vi.stubGlobal("fetch", mock);
  return calls;
}

function renderButton(policy: UpdatePolicy) {
  const calls = installFetchMock();
  const setPolicies = vi.fn();
  const showToast = vi.fn();
  render(
    <Wrapper>
      <PolicyActionButton
        containerName="nginx"
        getPolicy={() => policy}
        setPolicies={setPolicies}
        busy={false}
        showToast={showToast}
      />
    </Wrapper>,
  );
  return { calls, setPolicies, showToast };
}

function makePolicy(overrides: Partial<UpdatePolicy> = {}): UpdatePolicy {
  return {
    container: "nginx",
    action: "pull-restart",
    cleanup_old_image: false,
    rollback_on_failure: false,
    notify_events: true,
    ...overrides,
  };
}

describe("PolicyActionButton — overrides de la guarda por contenedor", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("precarga los overrides desde la política existente", async () => {
    renderButton(makePolicy({ max_start_attempts: 2, health_window_secs: 25 }));

    fireEvent.click(screen.getByRole("button", { name: /configurar/i }));

    expect(
      await screen.findByLabelText("Máximo de intentos de arranque"),
    ).toHaveValue("2");
    expect(screen.getByLabelText("Ventana de salud (s)")).toHaveValue("25");
  });

  it("envía max_start_attempts y health_window_secs en el PUT del contenedor", async () => {
    const { calls } = renderButton(makePolicy());

    fireEvent.click(screen.getByRole("button", { name: /configurar/i }));

    fireEvent.change(
      await screen.findByLabelText("Máximo de intentos de arranque"),
      { target: { value: "5" } },
    );
    fireEvent.change(screen.getByLabelText("Ventana de salud (s)"), {
      target: { value: "40" },
    });

    fireEvent.click(screen.getByRole("button", { name: /guardar política/i }));

    await waitFor(() => {
      const put = calls.find(
        (c) =>
          c.url === "/api/update-policies/nginx" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        max_start_attempts: 5,
        health_window_secs: 40,
      });
    });
  });

  it("sin overrides previos envía null (no resetea ni inventa valores)", async () => {
    const { calls } = renderButton(makePolicy());

    fireEvent.click(screen.getByRole("button", { name: /configurar/i }));
    await screen.findByLabelText("Máximo de intentos de arranque");

    fireEvent.click(screen.getByRole("button", { name: /guardar política/i }));

    await waitFor(() => {
      const put = calls.find(
        (c) => c.url === "/api/update-policies/nginx" && c.method === "PUT",
      );
      expect(put).toBeDefined();
      expect(put!.body).toMatchObject({
        max_start_attempts: null,
        health_window_secs: null,
      });
    });
  });
});
