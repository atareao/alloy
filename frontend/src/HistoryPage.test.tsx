import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
import { ConfigProvider } from "antd";
import HistoryPage from "./HistoryPage";
import { historyRowKey } from "./historyQuery";
import type { HistoryEntry, PagedHistory } from "./types";

/**
 * RED (task 3.1–3.8) — paginación en servidor, elipsis real y columna única de versión.
 *
 * Contrato que deben cumplir estos tests (aún NO implementado):
 *  - `HistoryPage` es dueña de su estado y de su consulta (D3): al montar pide
 *    `/api/history?page=1&page_size=10` y pinta los `items` con el `total` recibido.
 *  - `Pagination` con selector 10/20/50 en escritorio y móvil; cambiar de página
 *    consulta la página nueva y cambiar de tamaño resetea a la página 1.
 *  - `Container`/`Imagen` usan elipsis real (sin `slice` manual) con tooltip del valor completo.
 *  - Una única columna `Versión` (nueva encima de la antigua, ambas con tooltip; vacío ⇒ `-`).
 *  - Clave de fila estable: cambiar de página renderiza las entradas de la nueva página.
 *  - Al cambiar `refreshToken` se reconsulta la página actual; tras borrar, la página 1.
 *
 * En RED fallan por aserción (no por import): `HistoryPage` todavía no hace su
 * propio `fetch` ni tiene `Pagination`, columna `Versión` ni elipsis con tooltip.
 *
 * La migración de los tests de la guarda (`rolled-back` / `failed-stopped`) es
 * explícita: pasan de recibir `history` por props a alimentarse del `fetch`.
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

function makeEntries(n: number, prefix = "c"): HistoryEntry[] {
  return Array.from({ length: n }, (_, i) =>
    makeEntry({ container: `${prefix}-${i}` }),
  );
}

function envelope(
  items: HistoryEntry[],
  total: number,
  page: number,
  pageSize: number,
): PagedHistory {
  return { items, total, page, page_size: pageSize };
}

/**
 * Mock de `fetch` que responde a `/api/history` con el envelope paginado del
 * backend (D1): devuelve los `items` de la página pedida y el `total` indicado,
 * ecoando `page`/`page_size` de la URL. Los `DELETE` responden `ok`.
 */
function historyFetch(pages: Record<number, HistoryEntry[]>, total: number) {
  return vi.fn().mockImplementation((url: string, opts?: RequestInit) => {
    if (typeof url === "string" && url.startsWith("/api/history")) {
      if (opts?.method === "DELETE") {
        return Promise.resolve(new Response("ok", { status: 200 }));
      }
      const u = new URL(url, "http://localhost");
      const page = Number(u.searchParams.get("page") ?? 1);
      const pageSize = Number(u.searchParams.get("page_size") ?? 10);
      return Promise.resolve(
        new Response(
          JSON.stringify(
            envelope(pages[page] ?? [], total, page, pageSize),
          ),
        ),
      );
    }
    return Promise.resolve(new Response("ok"));
  });
}

/**
 * URLs de cada **lectura** (`GET`) a `/api/history` emitida contra el mock. El
 * `DELETE` de borrado también apunta a `/api/history` (sin query), así que se
 * excluye por método para no contarlo como lectura.
 */
function historyCalls(fetchMock: ReturnType<typeof vi.fn>): string[] {
  return fetchMock.mock.calls
    .filter((c) => (c[1] as RequestInit | undefined)?.method !== "DELETE")
    .map((c) => c[0] as string)
    .filter((u) => typeof u === "string" && u.startsWith("/api/history"));
}

/** Peticiones `DELETE` emitidas contra el mock. */
function deleteCalls(fetchMock: ReturnType<typeof vi.fn>) {
  return fetchMock.mock.calls.filter(
    (c) => (c[1] as RequestInit | undefined)?.method === "DELETE",
  );
}

/** Monta `HistoryPage` con la señal de refresco a 0 salvo que se indique otra. */
function renderPage(props: { refreshToken?: number } = {}) {
  return render(
    <Wrapper>
      <HistoryPage refreshToken={0} {...props} />
    </Wrapper>,
  );
}

async function tagFor(label: string | RegExp): Promise<HTMLElement> {
  const el = await screen.findByText(label);
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

/**
 * jsdom no calcula layout, así que antd nunca considera que un texto "desborda"
 * (`isEllipsis` es false) y desactiva el tooltip. Se emula `isEleEllipsis` de
 * `antd/es/typography/Base/util.js`: el nodo de medida
 * (`ant-typography-css-ellipsis-content-measure`, presente con NODE_ENV != production)
 * reporta un rect más ancho que el nodo, forzando la elipsis real y su tooltip.
 */
function mockOverflowRects() {
  const rect = (width: number) =>
    ({
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: width,
      bottom: 20,
      width,
      height: 20,
      toJSON: () => ({}),
    }) as DOMRect;
  return vi
    .spyOn(Element.prototype, "getBoundingClientRect")
    .mockImplementation(function (this: Element) {
      return this.classList.contains(
        "ant-typography-css-ellipsis-content-measure",
      )
        ? rect(900)
        : rect(200);
    });
}

/** Pasa el ratón por `target` y comprueba que aparece un tooltip con `value`. */
async function expectTooltipOn(target: Element, value: string) {
  fireEvent.mouseEnter(target);
  await waitFor(() => {
    const tips = screen.queryAllByRole("tooltip");
    expect(tips.some((t) => (t.textContent ?? "").includes(value))).toBe(true);
  });
}

/**
 * jsdom no calcula layout, así que no puede medir el truncado real (ancho visible
 * vs. ancho completo) — eso se verificó en un navegador real con Playwright. Lo que
 * sí resuelve jsdom es `getComputedStyle`, y por tanto la CAUSA del truncado: la
 * cota de ancho.
 *
 * antd aplica `max-width: 100%` a todo nodo con `ellipsis`, así que exigir «no es
 * `none`» es un proxy que pasa siempre: en una tabla `width: max-content` con
 * `table-layout: auto`, ese `100%` equivale al ancho completo del texto y NO trunca.
 * Por eso se exige una cota ABSOLUTA en píxeles, que es la que aporta la
 * implementación (y la única que trunca de verdad).
 */
function expectBoundedWidth(el: Element) {
  const { maxWidth } = window.getComputedStyle(el);
  expect(maxWidth).toMatch(/^\d+(\.\d+)?px$/);
  expect(parseFloat(maxWidth)).toBeLessThanOrEqual(320);
}

beforeEach(() => {
  vi.restoreAllMocks();
});

afterEach(() => {
  setMobileViewport(false);
});

// ── 3.1 Carga inicial contra el servidor ───────────────────────────
describe("HistoryPage — paginación en servidor (D1/D3)", () => {
  it("3.1 solicita /api/history?page=1&page_size=10 al montar y pinta items y total", async () => {
    const fetchMock = historyFetch({ 1: makeEntries(20) }, 25);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    await waitFor(() =>
      expect(historyCalls(fetchMock)[0]).toBe("/api/history?page=1&page_size=10"),
    );
    // Pinta los items de la respuesta (no una lista completa recortada en cliente).
    expect(await screen.findByText("c-0")).toBeInTheDocument();
    expect(screen.getByText("c-19")).toBeInTheDocument();
    // El total del control es el de la respuesta (25), no el de la página (20).
    expect(await screen.findByText("25 entradas")).toBeInTheDocument();
  });

  // ── 3.2 Pagination: página y tamaño ─────────────────────────────
  it("3.2 cambiar de página consulta page=2 y cambiar el tamaño a 50 resetea a page=1&page_size=50", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(5, "p2") },
      25,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText("25 entradas");

    fireEvent.click(screen.getByLabelText("Page 2"));
    await waitFor(() =>
      expect(historyCalls(fetchMock).some((u) => u.includes("page=2"))).toBe(
        true,
      ),
    );

    // Cambiar el tamaño a 50 resetea a la página 1.
    const select =
      document.querySelector(".ant-pagination-options .ant-select") ??
      document.querySelector(".ant-select");
    expect(select).not.toBeNull();
    fireEvent.mouseDown(select as Element);
    const option = await screen.findByTitle("50 / page");
    fireEvent.click(option);

    await waitFor(() =>
      expect(
        historyCalls(fetchMock).some(
          (u) => u.includes("page=1") && u.includes("page_size=50"),
        ),
      ).toBe(true),
    );
  });

  // ── 3.3 Pagination en móvil ─────────────────────────────────────
  it("3.3 el Pagination está presente en la vista móvil con los tamaños 10/20/50", async () => {
    setMobileViewport(true);
    const fetchMock = historyFetch({ 1: makeEntries(20) }, 25);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    // En móvil se renderizan tarjetas, no la tabla.
    expect(await screen.findByText("c-0")).toBeInTheDocument();
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
    expect(document.querySelector(".ant-pagination")).not.toBeNull();

    const select =
      document.querySelector(".ant-pagination-options .ant-select") ??
      document.querySelector(".ant-select");
    expect(select).not.toBeNull();
    fireEvent.mouseDown(select as Element);
    // antd 6 duplica `title="20 / page"` (valor seleccionado + opción), así que
    // no se puede usar getByTitle; comprobamos los nodos de opción del desplegable.
    await waitFor(() =>
      expect(
        document.querySelectorAll(".ant-select-item-option").length,
      ).toBeGreaterThanOrEqual(3),
    );
    const optionTitles = Array.from(
      document.querySelectorAll<HTMLElement>(".ant-select-item-option"),
    ).map((el) => el.getAttribute("title"));
    expect(optionTitles).toEqual(
      expect.arrayContaining(["10 / page", "20 / page", "50 / page"]),
    );
  });

  // ── 3.7 Clave de fila estable ───────────────────────────────────
  it("3.7 al cambiar de página se renderizan las entradas de la nueva página", async () => {
    const fetchMock = historyFetch(
      {
        1: [
          makeEntry({ container: "p1-a", timestamp: "2026-01-01T00:00:00Z" }),
          makeEntry({ container: "p1-b", timestamp: "2026-01-01T00:00:01Z" }),
        ],
        2: [
          makeEntry({ container: "p2-a", timestamp: "2026-01-02T00:00:00Z" }),
        ],
      },
      25,
    );
    vi.stubGlobal("fetch", fetchMock);

    // Con `rowKey={index}` React reutilizaría filas al cambiar de página y podría
    // emitir warnings de claves; se espía `console.error` para detectarlo.
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    renderPage();
    await screen.findByText("p1-a");
    expect(screen.getByText("p1-b")).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText("Page 2"));

    expect(await screen.findByText("p2-a")).toBeInTheDocument();
    // No se reutilizan filas de la página anterior.
    expect(screen.queryByText("p1-a")).not.toBeInTheDocument();
    expect(screen.queryByText("p1-b")).not.toBeInTheDocument();

    // React no emite warnings de claves duplicadas durante el cambio de página.
    const keyWarnings = errorSpy.mock.calls.filter((c) =>
      String(c[0] ?? "").includes("key"),
    );
    expect(keyWarnings).toHaveLength(0);

    errorSpy.mockRestore();
  });

  // ── 3.8 Refresco ────────────────────────────────────────────────
  it("3.8 al cambiar la señal de refresco se reconsulta la página actual", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(5, "p2") },
      25,
    );
    vi.stubGlobal("fetch", fetchMock);

    const { rerender } = renderPage();
    await screen.findByText("25 entradas");
    fireEvent.click(screen.getByLabelText("Page 2"));
    await waitFor(() =>
      expect(historyCalls(fetchMock).some((u) => u.includes("page=2"))).toBe(
        true,
      ),
    );

    const before = historyCalls(fetchMock).length;
    rerender(
      <Wrapper>
        <HistoryPage refreshToken={1} />
      </Wrapper>,
    );

    await waitFor(() =>
      expect(historyCalls(fetchMock).length).toBeGreaterThan(before),
    );
    expect(historyCalls(fetchMock).at(-1)).toBe(
      "/api/history?page=2&page_size=10",
    );
  });

  it("3.8 tras borrar el histórico se consulta la página 1", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(5, "p2") },
      25,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText("25 entradas");
    fireEvent.click(screen.getByLabelText("Page 2"));
    await waitFor(() =>
      expect(historyCalls(fetchMock).some((u) => u.includes("page=2"))).toBe(
        true,
      ),
    );

    const readsBeforeDelete = historyCalls(fetchMock).length;

    fireEvent.click(screen.getByText("Limpiar"));
    fireEvent.click(await screen.findByText("Eliminar todo"));

    await waitFor(() => expect(deleteCalls(fetchMock).length).toBeGreaterThan(0));
    // Tras borrar se reconsulta la página 1: exigimos una lectura NUEVA con
    // `page=1` posterior al borrado, no que la última llamada de cualquier tipo
    // (el propio `DELETE`) sea esa URL.
    await waitFor(() =>
      expect(
        historyCalls(fetchMock)
          .slice(readsBeforeDelete)
          .some((u) => u === "/api/history?page=1&page_size=10"),
      ).toBe(true),
    );
  });
});

// ── 3.7 Clave de fila estable (unitaria) ───────────────────────────
describe("historyRowKey — clave de fila estable derivada de los datos (D6)", () => {
  it("(a) dos entradas distintas producen claves distintas", () => {
    const a = makeEntry({ container: "a", timestamp: "2026-01-01T00:00:00Z" });
    const b = makeEntry({ container: "b", timestamp: "2026-01-01T00:00:01Z" });
    expect(historyRowKey(a)).not.toBe(historyRowKey(b));
  });

  it("(b) la misma entrada produce la misma clave en dos posiciones distintas", () => {
    const entry = makeEntry({ container: "same" });
    const atFirst = [entry, makeEntry({ container: "other" })];
    const atSecond = [makeEntry({ container: "other" }), entry];
    // La entrada en la posición 0 de una página y en la 1 de otra → misma clave.
    expect(historyRowKey(atFirst[0])).toBe(historyRowKey(atSecond[1]));
    // …y distinta de la de su vecina (la clave identifica la fila, no la posición).
    expect(historyRowKey(atFirst[0])).not.toBe(historyRowKey(atFirst[1]));
  });

  it("(c) la clave no es el índice ni depende de la posición", () => {
    const entry = makeEntry({ container: "stable" });
    const key = historyRowKey(entry);
    expect(key).not.toBe("0");
    expect(key).not.toBe("1");
    // Se deriva de los campos de la entrada, no de su posición en la lista.
    expect(key).toContain(entry.container);
    expect(key).toContain(entry.timestamp);
  });
});

// ── 3.4 Elipsis real con tooltip ───────────────────────────────────
describe("HistoryPage — elipsis real en Container e Imagen (D4)", () => {
  const LONG_CONTAINER =
    "very-long-container-name-that-does-not-fit-in-the-column";
  const LONG_IMAGE =
    "registry.example.com/team/very-long-image-name-that-does-not-fit:latest";

  it("3.4 Container e Imagen usan elipsis real (sin slice manual) y el tooltip lleva el valor completo", async () => {
    mockOverflowRects();
    const fetchMock = historyFetch(
      { 1: [makeEntry({ container: LONG_CONTAINER, image: LONG_IMAGE })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    const containerNode = (await screen.findByText(LONG_CONTAINER)).closest(
      ".ant-typography",
    );
    expect(containerNode).toHaveClass("ant-typography-ellipsis");
    // El valor completo sigue en el DOM: no hay recorte manual.
    expect(containerNode).toHaveTextContent(LONG_CONTAINER);
    expect(screen.queryByText(/\.\.\./)).toBeNull();
    // La celda está acotada: sin cota, el truncado real nunca se dispara.
    expectBoundedWidth(containerNode as Element);
    await expectTooltipOn(containerNode as Element, LONG_CONTAINER);

    const imageNode = screen.getByText(LONG_IMAGE).closest(".ant-typography");
    expect(imageNode).toHaveClass("ant-typography-ellipsis");
    expect(imageNode).toHaveTextContent(LONG_IMAGE);
    expectBoundedWidth(imageNode as Element);
    await expectTooltipOn(imageNode as Element, LONG_IMAGE);
  });

  it("3.4 la celda del contenedor no desborda con un nombre muy largo (cota de ancho + se puede encoger)", async () => {
    // jsdom no calcula layout: no se puede medir el truncado real (ancho visible
    // vs. ancho completo), eso se verificó en un navegador real. Lo que sí se
    // comprueba aquí es la causa del truncado: el nodo truncable tiene una cota de
    // ancho efectiva (sin ella, `max-width` computa a `none` y este test falla) y
    // puede encogerse (`minWidth: 0`) para que la elipsis de antd se aplique.
    const longName = "a".repeat(200);
    const fetchMock = historyFetch({ 1: [makeEntry({ container: longName })] }, 1);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    const node = (await screen.findByText(longName)).closest(".ant-typography");
    expect(node).not.toBeNull();
    expect(node).toHaveStyle({ minWidth: "0px" });
    expect(node).toHaveClass("ant-typography-ellipsis");
    expect(node).toHaveTextContent(longName);
    expectBoundedWidth(node as Element);

    // La columna `Imagen` lleva la misma cota (regresión de develop restaurada).
    const imageNode = screen
      .getByText("nginx:latest")
      .closest(".ant-typography");
    expect(imageNode).not.toBeNull();
    expectBoundedWidth(imageNode as Element);
  });
});

// ── 3.5 / 3.6 Columna única Versión ────────────────────────────────
describe("HistoryPage — columna única Versión (D7)", () => {
  const OLD_DIGEST = `sha256:${"a".repeat(64)}`;
  const NEW_DIGEST = `sha256:${"b".repeat(64)}`;

  it("3.5 existe una única columna Versión, el digest nuevo va encima del anterior y ambos con tooltip", async () => {
    mockOverflowRects();
    const fetchMock = historyFetch(
      { 1: [makeEntry({ old_digest: OLD_DIGEST, new_digest: NEW_DIGEST })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText(NEW_DIGEST);

    // Una sola columna titulada `Versión`; ya no existen las columnas separadas.
    expect(screen.getAllByText("Versión")).toHaveLength(1);
    expect(screen.queryByText("Versión anterior")).not.toBeInTheDocument();
    expect(screen.queryByText("Nueva versión")).not.toBeInTheDocument();

    const newNode = screen.getByText(NEW_DIGEST);
    const oldNode = screen.getByText(OLD_DIGEST);
    // El digest nuevo aparece ANTES (encima) del anterior en el DOM.
    expect(
      newNode.compareDocumentPosition(oldNode) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    await expectTooltipOn(newNode, NEW_DIGEST);
    await expectTooltipOn(oldNode, OLD_DIGEST);
  });

  it("3.5 un digest anterior vacío se muestra como -", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ old_digest: "", new_digest: NEW_DIGEST })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText(NEW_DIGEST);

    expect(screen.getAllByText("-").length).toBeGreaterThanOrEqual(1);
  });

  it("3.6 la tarjeta móvil muestra la misma composición de versión (nueva encima de la anterior)", async () => {
    setMobileViewport(true);
    const fetchMock = historyFetch(
      { 1: [makeEntry({ old_digest: OLD_DIGEST, new_digest: NEW_DIGEST })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    // En móvil se renderiza la tarjeta, no la tabla.
    expect(screen.queryByRole("table")).not.toBeInTheDocument();
    const newNode = await screen.findByText(NEW_DIGEST);
    const oldNode = screen.getByText(OLD_DIGEST);
    expect(
      newNode.compareDocumentPosition(oldNode) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });
});

// ── Guarda: estados rolled-back / failed-stopped (migrados a fetch) ─
describe("HistoryPage — estados de la guarda (rolled-back / failed-stopped)", () => {
  it("rolled-back: etiqueta propia, color warning y tooltip explicativo", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ status: "rolled-back" })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    const tag = await tagFor("↩️ Rollback aplicado");
    expect(tag).toHaveClass("ant-tag-warning");

    fireEvent.mouseEnter(tag);
    await waitFor(() => {
      expect(document.body.textContent ?? "").toMatch(
        /se restauró la imagen anterior/i,
      );
    });
  });

  it("failed-stopped: etiqueta propia, color error y tooltip explicativo", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ status: "failed-stopped" })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    const tag = await tagFor("🛑 Fallo terminal");
    expect(tag).toHaveClass("ant-tag-error");

    fireEvent.mouseEnter(tag);
    await waitFor(() => {
      expect(document.body.textContent ?? "").toMatch(
        /el contenedor quedó parado/i,
      );
    });
  });

  it("muestra el número de intentos de arranque en una columna propia", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ status: "rolled-back", attempts: 7, duration_ms: 1000 })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    expect(await screen.findByText("Intentos")).toBeInTheDocument();
    expect(screen.getByText("7")).toBeInTheDocument();
  });

  it("mapea update-check-stack como estado de comprobación (D8)", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ status: "update-check-stack" })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    expect(await screen.findByText("📦 update-check")).toBeInTheDocument();
  });
});

// ── 3.10 UI de filtros: búsqueda y estado ──────────────────────────
describe("HistoryPage — filtros de texto y estado (servidor)", () => {
  const SEARCH_PLACEHOLDER = "Buscar por contenedor, imagen o digest...";

  it("3.10 el buscador aplica debounce, emite q y resetea a la página 1", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(20, "p2") },
      40,
    );
    vi.stubGlobal("fetch", fetchMock);

    // El debounce usa un `setTimeout` de 300 ms: con timers reales el disparo es
    // no determinista (fallos intermitentes y avisos de `act`). Se activan fake
    // timers *después* de montar y de cambiar de página (que usan `waitFor` con
    // timers reales) y se avanza el reloj explícitamente dentro de `act`.
    renderPage();
    await screen.findByText("40 entradas");

    fireEvent.click(screen.getByLabelText("Page 2"));
    await waitFor(() =>
      expect(historyCalls(fetchMock).at(-1)).toContain("page=2"),
    );

    vi.useFakeTimers();
    try {
      const before = historyCalls(fetchMock).length;
      fireEvent.change(screen.getByPlaceholderText(SEARCH_PLACEHOLDER), {
        target: { value: "web" },
      });

      // Debounce de 300 ms: sin avanzar el reloj no hay petición con `q=web`.
      expect(historyCalls(fetchMock).length).toBe(before);
      expect(historyCalls(fetchMock).some((u) => u.includes("q=web"))).toBe(
        false,
      );

      // Al asentarse el debounce se emite `q=web` sobre la página 1.
      await act(async () => {
        vi.advanceTimersByTime(300);
      });

      expect(historyCalls(fetchMock).at(-1)).toBe(
        "/api/history?page=1&page_size=10&q=web",
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it("3.10 el selector de estado emite status repetido y resetea a la página 1", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(20, "p2") },
      40,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText("40 entradas");

    fireEvent.click(screen.getByLabelText("Page 2"));
    await waitFor(() =>
      expect(historyCalls(fetchMock).at(-1)).toContain("page=2"),
    );

    fireEvent.click(screen.getByText("Con error"));
    fireEvent.click(screen.getByText("Rollback"));

    await waitFor(() =>
      expect(historyCalls(fetchMock).at(-1)).toBe(
        "/api/history?page=1&page_size=10&status=error&status=rolled-back",
      ),
    );
  });

  it("3.10 sin filtros la consulta sólo lleva page y page_size", async () => {
    const fetchMock = historyFetch({ 1: makeEntries(3) }, 3);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await waitFor(() =>
      expect(historyCalls(fetchMock).length).toBeGreaterThan(0),
    );

    expect(historyCalls(fetchMock)[0]).toBe("/api/history?page=1&page_size=10");
    expect(
      historyCalls(fetchMock).every(
        (u) => !u.includes("q=") && !u.includes("status="),
      ),
    ).toBe(true);
  });

  it("3.10 pinta todos los items recibidos sin filtrar en cliente", async () => {
    const items = makeEntries(3, "keep");
    const fetchMock = historyFetch({ 1: items }, 3);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    expect(await screen.findByText("keep-0")).toBeInTheDocument();
    expect(screen.getByText("keep-1")).toBeInTheDocument();
    expect(screen.getByText("keep-2")).toBeInTheDocument();
  });
});

// ── 3.11 Accesibilidad de los chips de estado ──────────────────────
describe("HistoryPage — chips de estado accesibles (a11y)", () => {
  it("3.11 el chip se activa con teclado (foco + Enter) y expone aria-pressed", async () => {
    const fetchMock = historyFetch({ 1: makeEntries(3) }, 3);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText("3 entradas");

    const chip = await tagFor("Con error");
    // Es un botón alcanzable por teclado, no un `span` inerte.
    expect(chip).toHaveAttribute("role", "button");
    expect(chip).toHaveAttribute("tabindex", "0");
    expect(chip).toHaveAttribute("aria-pressed", "false");

    chip.focus();
    expect(chip).toHaveFocus();

    fireEvent.keyDown(chip, { key: "Enter" });
    await waitFor(() =>
      expect(chip).toHaveAttribute("aria-pressed", "true"),
    );
    await waitFor(() =>
      expect(historyCalls(fetchMock).at(-1)).toContain("status=error"),
    );

    // `Space` también activa el chip (alternándolo de nuevo).
    fireEvent.keyDown(chip, { key: " " });
    await waitFor(() =>
      expect(chip).toHaveAttribute("aria-pressed", "false"),
    );
  });
});

describe("HistoryPage — tarjeta móvil (fondo coherente con el nivel del estado)", () => {
  beforeEach(() => {
    setMobileViewport(true);
  });

  it("rolled-back: tarjeta con fondo de aviso (warning), no de error, y etiqueta propia", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ status: "rolled-back" })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    // En móvil se renderiza la tarjeta, no la tabla.
    expect(screen.queryByRole("table")).not.toBeInTheDocument();

    const tag = await tagFor("↩️ Rollback aplicado");
    expect(tag).toHaveClass("ant-tag-warning");

    const card = tag.closest(".ant-card") as HTMLElement | null;
    expect(card).not.toBeNull();
    const style = card!.getAttribute("style") ?? "";
    expect(style).toContain("--ant-color-warning-bg");
    expect(style).not.toContain("--ant-color-error-bg");
  });

  it("failed-stopped: tarjeta con fondo de error y etiqueta de error", async () => {
    const fetchMock = historyFetch(
      { 1: [makeEntry({ status: "failed-stopped" })] },
      1,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    const tag = await tagFor("🛑 Fallo terminal");
    expect(tag).toHaveClass("ant-tag-error");

    const card = tag.closest(".ant-card") as HTMLElement | null;
    const style = card?.getAttribute("style") ?? "";
    expect(style).toContain("--ant-color-error-bg");
  });
});

// ── Regresión: el debounce no debe resetear la página al montar ────
// El efecto de debounce corría también en el montaje (con `searchQuery` ya en
// `""`), programando un `setPage(1)` a los 300 ms que pisaba cualquier cambio de
// página inmediato. Estos tests fijan que el debounce sólo actúa cuando el texto
// cambia de verdad.
describe("HistoryPage — regresión: el debounce no resetea la página al montar", () => {
  const SEARCH_PLACEHOLDER = "Buscar por contenedor, imagen o digest...";

  // Con fake timers `waitFor`/`findByText` de Testing Library se atascan (no
  // detectan los timers de Vitest), así que se vacía la cola de promesas del
  // `fetch` simulado con `act` y se avanza el reloj explícitamente.
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("cambiar de página justo tras el montaje mantiene la página 2 (sin lectura posterior con page=1)", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(5, "p2") },
      25,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await act(async () => {}); // fetch inicial (página 1)
    expect(screen.getByText("25 entradas")).toBeInTheDocument();

    // Cambio de página inmediato, dentro de la ventana de 300 ms del debounce.
    fireEvent.click(screen.getByLabelText("Page 2"));
    await act(async () => {}); // fetch de la página 2
    expect(historyCalls(fetchMock).at(-1)).toBe(
      "/api/history?page=2&page_size=10",
    );

    // Se deja pasar la ventana del debounce: ningún temporizador del montaje
    // debe disparar un `page=1` tardío.
    await act(async () => {
      vi.advanceTimersByTime(350);
    });

    const reads = historyCalls(fetchMock);
    const idxPage2 = reads.findIndex((u) => u.includes("page=2"));
    expect(idxPage2).toBeGreaterThanOrEqual(0);
    expect(reads.slice(idxPage2).some((u) => u.includes("page=1"))).toBe(false);
    expect(reads.at(-1)).toBe("/api/history?page=2&page_size=10");
  });

  it("escribir y borrar dentro de la ventana del debounce no resetea la página 2", async () => {
    const fetchMock = historyFetch(
      { 1: makeEntries(20, "p1"), 2: makeEntries(20, "p2") },
      40,
    );
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await act(async () => {}); // fetch inicial (página 1)
    fireEvent.click(screen.getByLabelText("Page 2"));
    await act(async () => {}); // fetch de la página 2
    expect(historyCalls(fetchMock).at(-1)).toContain("page=2");

    // Escribir y borrar rápido: el texto neto vuelve a `""`, así que el debounce
    // no debe reasentarse ni devolver la página a 1.
    const input = screen.getByPlaceholderText(SEARCH_PLACEHOLDER);
    fireEvent.change(input, { target: { value: "web" } });
    fireEvent.change(input, { target: { value: "" } });

    await act(async () => {
      vi.advanceTimersByTime(350);
    });

    const reads = historyCalls(fetchMock);
    expect(reads.some((u) => u.includes("q=web"))).toBe(false);
    expect(reads.at(-1)).toBe("/api/history?page=2&page_size=10");
  });
});

// ── Mensaje de "sin resultados" según haya filtros o no ────────────
describe("HistoryPage — mensaje de vacío según filtros", () => {
  const SEARCH_PLACEHOLDER = "Buscar por contenedor, imagen o digest...";
  const EMPTY_HISTORY = /No hay historial de actualizaciones/;
  const NO_MATCHES = /No hay resultados para los filtros aplicados/;

  it("sin filtros y sin resultados muestra el mensaje de historial vacío", async () => {
    const fetchMock = historyFetch({}, 0);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    expect(await screen.findByText(EMPTY_HISTORY)).toBeInTheDocument();
    expect(screen.queryByText(NO_MATCHES)).not.toBeInTheDocument();
  });

  it("con filtro de texto activo y sin resultados muestra el mensaje de filtros", async () => {
    const fetchMock = historyFetch({}, 0);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();
    await screen.findByText(EMPTY_HISTORY);

    // Se activan fake timers para el debounce y se vacía la cola de promesas del
    // `fetch` con `act` (con fake timers `findByText`/`waitFor` se atascan).
    vi.useFakeTimers();
    try {
      fireEvent.change(screen.getByPlaceholderText(SEARCH_PLACEHOLDER), {
        target: { value: "web" },
      });
      await act(async () => {
        vi.advanceTimersByTime(300);
      });

      expect(screen.getByText(NO_MATCHES)).toBeInTheDocument();
      expect(screen.queryByText(EMPTY_HISTORY)).not.toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });
});

// ── pageSize por defecto del Historial (ui-pagination) ─────────────
/**
 * Historial con paginación por defecto de 10.
 *
 * Contrato: `HistoryPage` arranca con `DEFAULT_HISTORY_PAGE_SIZE = 10`
 * (definido en `historyQuery.ts`), de modo que la petición inicial es
 * `page_size=10` y se pintan 10 filas.
 */
describe("HistoryPage — pageSize por defecto 10", () => {
  it("la petición inicial usa page_size=10 y muestra 10 filas en la página 1", async () => {
    const fetchMock = historyFetch({ 1: makeEntries(10) }, 25);
    vi.stubGlobal("fetch", fetchMock);

    renderPage();

    await waitFor(() =>
      expect(historyCalls(fetchMock)[0]).toBe(
        "/api/history?page=1&page_size=10",
      ),
    );

    // 10 filas (c-0..c-9) y ninguna más (la elipsis recorta la página 2).
    expect(await screen.findByText("c-0")).toBeInTheDocument();
    expect(screen.getByText("c-9")).toBeInTheDocument();
    expect(screen.queryByText("c-10")).not.toBeInTheDocument();

    // El paginador refleja el total (25) y está en la página 1.
    expect(await screen.findByText("25 entradas")).toBeInTheDocument();
    expect(
      document.querySelector(".ant-pagination-item-active")?.textContent,
    ).toBe("1");
  });
});
