import { describe, it, expect, vi } from "vitest";
import { useEffect, useState } from "react";
import {
  render,
  screen,
  waitFor,
  fireEvent,
  act,
  cleanup,
} from "@testing-library/react";
import { ConfigProvider, message, notification } from "antd";
import App from "./App";
import { EventSourceMock, flushReactWork } from "./test-setup";

/**
 * Regresión del fallo intermitente del job `frontend`:
 * `ReferenceError: window is not defined` en `react-dom-client.development.js` al destruir
 * el entorno jsdom. Causa raíz: la API estática de antd (`notification.success`) monta un
 * root de React propio en `document.body` que `cleanup()` no desmonta, y su *flush* de
 * *passive effects* queda agendado como macrotarea más allá del test.
 *
 * Estos tests son deliberadamente deterministas: no intentan reproducir el race (que sólo
 * se manifiesta con la CPU saturada), sino el invariante que lo hace imposible.
 */

// Bandera de precondición: evita un falso verde cuando el test B se ejecuta aislado
// (sin que antes haya corrido el test A que dispara la notificación estática).
let sawStaticNotification = false;

function mockFetch() {
  return vi.fn().mockImplementation((url: string) => {
    if (url === "/api/auth/me") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            authenticated: true,
            user: { sub: "test", name: "Test User", email: "test@example.com" },
          }),
          { status: 200 },
        ),
      );
    }
    if (url === "/api/containers") {
      return Promise.resolve(new Response(JSON.stringify([])));
    }
    if (url === "/api/state") {
      // Nunca resuelve: evita el bucle infinito del long-poll durante los tests.
      return new Promise(() => {});
    }
    if (url === "/api/history") {
      return Promise.resolve(new Response(JSON.stringify([])));
    }
    if (url === "/api/config") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            host: "localhost",
            port: 3066,
            scan_interval_secs: 30,
            allowed_containers: [],
            oidc_issuer_url: "https://issuer.example.com",
            oidc_client_id: "test",
            oidc_redirect_url: "http://localhost:5173/api/auth/callback",
            auto_update_interval_hours: 6,
            telegram_bot_token: "",
            telegram_chat_id: "",
            matrix_homeserver: "",
            matrix_user: "",
            matrix_password: "",
            matrix_room_id: "",
          }),
        ),
      );
    }
    if (url === "/api/update-check/config") {
      return Promise.resolve(
        new Response(
          JSON.stringify({
            last_check: null,
            next_check: null,
            auto_update_interval_hours: 6,
          }),
        ),
      );
    }
    if (url === "/api/check-all") {
      return Promise.resolve(new Response(JSON.stringify([])));
    }
    return Promise.resolve(new Response("ok"));
  });
}

describe("aislamiento de la suite: holder estático de antd", () => {
  it("dispara una notificación estática de antd (precondición del invariante)", async () => {
    vi.stubGlobal("fetch", mockFetch());
    EventSourceMock.reset();

    render(
      <ConfigProvider>
        <App colorScheme="dark" setColorScheme={() => {}} />
      </ConfigProvider>,
    );

    await waitFor(() => {
      expect(screen.getByText("Alloy")).toBeInTheDocument();
    });

    // "Check" arranca el batch; el evento `update-progress` con `done: true` lo cierra y
    // dispara `notification.success(...)` (API estática de antd).
    fireEvent.click(screen.getByText("Check"));
    act(() => {
      EventSourceMock.emit("update-progress", {
        container: "nginx",
        status: "done",
        done: true,
        error: null,
        total: 2,
        checked: 2,
        updated: 1,
        errors: 0,
      });
    });

    await waitFor(() => {
      expect(
        document.querySelector(".ant-notification-notice"),
      ).toBeInTheDocument();
    });

    sawStaticNotification = true;
  });

  it("no deja el holder de antd montado al empezar el siguiente test", () => {
    // Falla en claro si este test se ejecuta aislado (el test A no disparó la notificación).
    expect(sawStaticNotification).toBe(true);

    // El `afterEach` global de `test-setup.ts` debe haber cerrado y desmontado el holder.
    expect(document.querySelector(".ant-notification")).toBeNull();
    expect(document.querySelector(".ant-notification-notice")).toBeNull();
    expect(document.querySelector(".ant-message")).toBeNull();
    expect(document.body.childElementCount).toBe(0);
  });
});

describe("flushReactWork", () => {
  it("drena macrotareas pendientes, no sólo microtareas", async () => {
    let ran = false;
    setImmediate(() => {
      ran = true;
    });

    await flushReactWork();

    expect(ran).toBe(true);
  });

  it("deja el scheduler de React sin trabajo: un macrotask extra no muta el DOM", async () => {
    function Probe() {
      const [n, setN] = useState(0);
      useEffect(() => {
        (globalThis as unknown as { __bump?: () => void }).__bump = () =>
          setN((v) => v + 1);
        return () => {
          delete (globalThis as unknown as { __bump?: () => void }).__bump;
        };
      }, []);
      return <span data-testid="probe">{n}</span>;
    }

    render(<Probe />);
    await waitFor(() => {
      expect(screen.getByTestId("probe").textContent).toBe("0");
    });

    // Actualización disparada FUERA de `act`: React la agenda como trabajo pendiente.
    (globalThis as unknown as { __bump: () => void }).__bump();

    await flushReactWork();

    // El flush ha drenado de verdad: la actualización agendada fuera de `act` ya se refleja.
    expect(screen.getByTestId("probe").textContent).toBe("1");

    // Tras el drenado no debe quedar nada agendado: un macrotask adicional no puede
    // provocar mutaciones en el DOM.
    let mutated = false;
    const observer = new MutationObserver(() => {
      mutated = true;
    });
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      characterData: true,
    });
    await new Promise<void>((resolve) => setImmediate(resolve));
    observer.disconnect();

    expect(mutated).toBe(false);
  });
});

describe("cierre global de la suite", () => {
  it("cierre idempotente: repetir el teardown no lanza y deja el DOM limpio", async () => {
    render(<span data-testid="probe-idem">x</span>);
    expect(screen.getByTestId("probe-idem")).toBeInTheDocument();

    // El `afterEach` global ya ejecutará esto una vez; aquí forzamos una segunda
    // pasada explícita para comprobar que el cierre es idempotente y no lanza.
    notification.destroy();
    message.destroy();
    cleanup();
    await flushReactWork();

    notification.destroy();
    message.destroy();
    cleanup();
    await flushReactWork();

    expect(document.body.childElementCount).toBe(0);
  });
});
