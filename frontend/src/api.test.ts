import { describe, it, expect, vi, beforeEach } from "vitest";
import { apiFetch, formatBytes } from "./api";

describe("formatBytes", () => {
  it('returns "0 B" for zero', () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats bytes", () => {
    expect(formatBytes(500)).toBe("500.00 B");
  });

  it("formats kilobytes", () => {
    expect(formatBytes(1024)).toBe("1.00 KB");
  });

  it("formats megabytes", () => {
    expect(formatBytes(1_048_576)).toBe("1.00 MB");
  });

  it("formats gigabytes", () => {
    expect(formatBytes(1_073_741_824)).toBe("1.00 GB");
  });

  it("formats terabytes", () => {
    expect(formatBytes(1_099_511_627_776)).toBe("1.00 TB");
  });

  it("rounds to two decimal places", () => {
    expect(formatBytes(1_500_000)).toBe("1.43 MB");
  });

  it("handles large values", () => {
    const result = formatBytes(5_000_000_000_000);
    expect(result).toMatch(/^\d+\.\d+ [KMGTP]B$/);
  });
});

describe("apiFetch", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("sends GET request with credentials include", async () => {
    const mockFetch = vi.fn().mockResolvedValue(new Response("ok"));
    vi.stubGlobal("fetch", mockFetch);

    await apiFetch("/api/health");

    expect(mockFetch).toHaveBeenCalledWith("/api/health", {
      credentials: "include",
      headers: {},
    });
  });

  it("merges custom headers", async () => {
    const mockFetch = vi.fn().mockResolvedValue(new Response("ok"));
    vi.stubGlobal("fetch", mockFetch);

    await apiFetch("/api/data", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    expect(mockFetch).toHaveBeenCalledWith("/api/data", {
      credentials: "include",
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
  });

  it("preserves existing opts besides headers", async () => {
    const mockFetch = vi.fn().mockResolvedValue(new Response("ok"));
    vi.stubGlobal("fetch", mockFetch);

    await apiFetch("/api/data", { method: "DELETE" });

    expect(mockFetch).toHaveBeenCalledWith("/api/data", {
      credentials: "include",
      method: "DELETE",
      headers: {},
    });
  });
});
