import { describe, it, expect, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useConfigSync } from "./useConfigSync";

interface Config {
  id: string;
}

describe("useConfigSync", () => {
  it("no llama a sync cuando la prop inicial es null", () => {
    const sync = vi.fn();
    renderHook(() => useConfigSync<Config>(null, sync));
    expect(sync).not.toHaveBeenCalled();
  });

  it("llama a sync una vez con la prop cuando llega una prop no nula", () => {
    const sync = vi.fn();
    const config: Config = { id: "a" };
    const { rerender } = renderHook<void, { configProp: Config | null }>(
      ({ configProp }) => useConfigSync(configProp, sync),
      { initialProps: { configProp: null } },
    );

    expect(sync).not.toHaveBeenCalled();
    rerender({ configProp: config });
    expect(sync).toHaveBeenCalledTimes(1);
    expect(sync).toHaveBeenCalledWith(config);
  });

  it("no vuelve a llamar a sync al re-renderizar con la misma prop", () => {
    const sync = vi.fn();
    const config: Config = { id: "a" };
    const { rerender } = renderHook(
      ({ configProp }: { configProp: Config | null }) =>
        useConfigSync(configProp, sync),
      { initialProps: { configProp: config } },
    );

    expect(sync).toHaveBeenCalledTimes(1);
    rerender({ configProp: config });
    expect(sync).toHaveBeenCalledTimes(1);
  });

  it("llama a sync con la nueva prop cuando cambia la identidad", () => {
    const sync = vi.fn();
    const first: Config = { id: "a" };
    const second: Config = { id: "b" };
    const { rerender } = renderHook(
      ({ configProp }: { configProp: Config | null }) =>
        useConfigSync(configProp, sync),
      { initialProps: { configProp: first } },
    );

    expect(sync).toHaveBeenCalledTimes(1);
    rerender({ configProp: second });
    expect(sync).toHaveBeenCalledTimes(2);
    expect(sync).toHaveBeenLastCalledWith(second);
  });

  it("vuelve a sincronizar al desmontar y remontar", () => {
    const sync = vi.fn();
    const config: Config = { id: "a" };
    const { unmount } = renderHook(() => useConfigSync<Config>(config, sync));

    expect(sync).toHaveBeenCalledTimes(1);
    unmount();

    const remounted = renderHook(() => useConfigSync<Config>(config, sync));
    expect(sync).toHaveBeenCalledTimes(2);
    expect(sync).toHaveBeenLastCalledWith(config);
    remounted.unmount();
  });
});
