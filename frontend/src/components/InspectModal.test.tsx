import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigProvider } from "antd";
import InspectModal from "./InspectModal";
import { makeInspectData } from "./testFixtures";
import type { InspectData } from "../types";

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ConfigProvider>{children}</ConfigProvider>;
}

interface SetupOptions {
  opened?: boolean;
  loading?: boolean;
  error?: string | null;
  inspectData?: InspectData | null;
}

function setup({
  opened = true,
  loading = false,
  error = null,
  inspectData = null,
}: SetupOptions = {}) {
  const onClose = vi.fn();

  render(
    <Wrapper>
      <InspectModal
        opened={opened}
        onClose={onClose}
        containerName="web"
        containerInfo={{
          image: "nginx",
          image_tag: "latest",
          registry_url: "https://registry.example.com/nginx",
        }}
        inspectData={inspectData}
        loading={loading}
        error={error}
      />
    </Wrapper>,
  );

  return { onClose };
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
});
