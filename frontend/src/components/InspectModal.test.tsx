import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import type { ComponentProps } from "react";
import InspectModal from "./InspectModal";
import { makeInspectData } from "./testFixtures";
import type { InspectData } from "../types";

type ContainerInfoProp = ComponentProps<typeof InspectModal>["containerInfo"];

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface SetupOptions {
  opened?: boolean;
  loading?: boolean;
  error?: string | null;
  inspectData?: InspectData | null;
  containerInfo?: ContainerInfoProp;
}

function setup({
  opened = true,
  loading = false,
  error = null,
  inspectData = null,
  containerInfo = {
    image: "nginx",
    image_tag: "latest",
    registry_url: "https://registry.example.com/nginx",
  },
}: SetupOptions = {}) {
  const onClose = vi.fn();

  render(
    <Wrapper>
      <InspectModal
        opened={opened}
        onClose={onClose}
        containerName="web"
        containerInfo={containerInfo}
        inspectData={inspectData}
        loading={loading}
        error={error}
      />
    </Wrapper>,
  );

  return { onClose };
}

/** Devuelve el div de valor de la fila `label` (debe ser único en el DOM). */
function getInfoRowValue(label: string): HTMLElement {
  const row = screen.getByText(label).closest(".ant-flex");
  const value = row?.querySelector<HTMLElement>(
    '[data-testid="info-row-value"]',
  );
  if (!value) {
    throw new Error(`No se encontró el valor de la fila «${label}»`);
  }
  return value;
}

describe("InspectModal", () => {
  it("muestra el estado de carga", () => {
    setup({ loading: true });
    expect(screen.getByText("Obteniendo información...")).toBeInTheDocument();
  });

  it("muestra el error y permite cerrar", () => {
    const { onClose } = setup({ error: "No se pudo obtener información" });

    expect(screen.getByText("No se pudo obtener información")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("muestra el estado vacío cuando no hay datos", () => {
    setup();
    expect(
      screen.getByText("Selecciona un container para inspeccionar"),
    ).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "General" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Puertos" })).toBeInTheDocument();
  });

  it("muestra los datos generales del contenedor", () => {
    setup({ inspectData: makeInspectData() });

    expect(screen.getByText("abc123")).toBeInTheDocument();
    expect(screen.getByText("nginx:latest")).toBeInTheDocument();
    expect(screen.getByText("unless-stopped")).toBeInTheDocument();
    expect(screen.getByText("healthy")).toBeInTheDocument();
    expect(screen.getByText("Información adicional")).toBeInTheDocument();
    expect(screen.getByText("Ver en registry")).toBeInTheDocument();
  });

  it("navega por las pestañas de puertos, volúmenes, redes, env y labels", () => {
    setup({ inspectData: makeInspectData() });

    fireEvent.click(screen.getByRole("tab", { name: "Puertos" }));
    expect(screen.getByText("8080")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Volúmenes" }));
    expect(screen.getByText("/host/data")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Redes" }));
    expect(screen.getByText("172.17.0.2")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "ENV" }));
    expect(screen.getByText("FOO=bar")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Labels" }));
    expect(screen.getByText("com.example.role:")).toBeInTheDocument();
  });

  it("cierra con el botón de cierre", () => {
    const { onClose } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  describe("imagen del contenedor", () => {
    const IMAGEN_LARGA =
      "registry.example.com/equipo/proyecto/imagen-con-nombre-extraordinariamente-largo:2026.11.22";

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("trunca la imagen principal con elipsis y conserva el valor completo", () => {
      setup({ inspectData: makeInspectData({ image: IMAGEN_LARGA }) });

      const nodoImagen = screen
        .getByText(IMAGEN_LARGA)
        .closest(".ant-typography");

      expect(nodoImagen).not.toBeNull();
      expect(nodoImagen).toHaveClass("ant-typography-ellipsis");
      expect(nodoImagen).toHaveTextContent(IMAGEN_LARGA);
    });

    it("ofrece el valor completo como tooltip cuando la imagen no cabe", async () => {
      const rect = (right: number) =>
        ({
          x: 0,
          y: 0,
          top: 0,
          left: 0,
          right,
          bottom: 20,
          width: right,
          height: 20,
          toJSON: () => ({}),
        }) as DOMRect;

      // Emula `isEleEllipsis` de `antd/es/typography/Base/util.js`: antd mide un
      // `<em class="ant-typography-css-ellipsis-content-measure">` (clase de
      // depuración presente solo con `NODE_ENV !== "production"`, antd 6.7.1) y
      // compara su rect con el del nodo para decidir si hay desbordamiento.
      // jsdom no calcula layout, así que forzamos rects con desbordamiento.
      vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(
        function (this: Element) {
          return this.classList.contains(
            "ant-typography-css-ellipsis-content-measure",
          )
            ? rect(900)
            : rect(200);
        },
      );

      setup({ inspectData: makeInspectData({ image: IMAGEN_LARGA }) });

      const nodoImagen = screen
        .getByText(IMAGEN_LARGA)
        .closest(".ant-typography");
      fireEvent.mouseEnter(nodoImagen as Element);

      const tooltip = await screen.findByRole("tooltip");
      expect(tooltip).toHaveTextContent(IMAGEN_LARGA);
    });

    it("usa el fallback image:tag cuando inspectData.image está vacío", () => {
      setup({
        inspectData: makeInspectData({ image: "" }),
        containerInfo: {
          image: "redis",
          image_tag: "7-alpine",
          registry_url: "",
        },
      });

      expect(screen.getByText("redis:7-alpine")).toBeInTheDocument();
    });

    it("hace encogible la imagen (minWidth: 0) para que la elipsis no desborde", () => {
      setup({ inspectData: makeInspectData({ image: IMAGEN_LARGA }) });

      const nodoImagen = screen
        .getByText(IMAGEN_LARGA)
        .closest(".ant-typography");
      expect(nodoImagen).not.toBeNull();
      expect(nodoImagen).toHaveStyle({ minWidth: "0px" });

      const valor = screen
        .getByText(IMAGEN_LARGA)
        .closest<HTMLElement>('[data-testid="info-row-value"]');
      expect(valor).not.toBeNull();
      expect(valor).toHaveStyle({ flex: "1", minWidth: "0px" });
    });

    it("renderiza sin errores y deja el valor de la imagen vacío sin containerInfo", () => {
      setup({ inspectData: makeInspectData({ image: "" }), containerInfo: null });

      const valor = getInfoRowValue("Imagen:");
      expect(valor.textContent).toBe("");
    });
  });

  describe("información adicional", () => {
    it("no muestra el bloque ni la fila de registry sin containerInfo", () => {
      setup({ inspectData: makeInspectData(), containerInfo: null });

      expect(screen.queryByText("Información adicional")).not.toBeInTheDocument();
      expect(screen.queryByText("Registry:")).not.toBeInTheDocument();
    });

    it("no muestra el enlace al registry cuando registry_url está vacío", () => {
      setup({
        inspectData: makeInspectData(),
        containerInfo: {
          image: "redis",
          image_tag: "7-alpine",
          registry_url: "",
        },
      });

      expect(screen.getByText("Información adicional")).toBeInTheDocument();
      expect(screen.queryByText("Ver en registry")).not.toBeInTheDocument();
      expect(screen.queryByText("Registry:")).not.toBeInTheDocument();
    });

    it("muestra el enlace al registry con registry_url no vacío", () => {
      setup({
        inspectData: makeInspectData(),
        containerInfo: {
          image: "redis",
          image_tag: "7-alpine",
          registry_url: "https://registry.example.com/redis",
        },
      });

      const enlace = screen.getByRole("link", { name: "Ver en registry" });
      expect(enlace).toHaveAttribute(
        "href",
        "https://registry.example.com/redis",
      );
    });
  });
});
