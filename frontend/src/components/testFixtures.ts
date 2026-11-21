import type { BatchProgress, ContainerInfo, InspectData } from "../types";

/**
 * Fixtures compartidos por los tests de los componentes del dashboard.
 * Sólo viven en el árbol de tests (no se importan desde producción).
 */
export function makeContainer(
  overrides: Partial<ContainerInfo> = {},
): ContainerInfo {
  const name = overrides.name ?? "web";
  return {
    id: `id-${name}`,
    name,
    image: "nginx",
    image_tag: "latest",
    size_mb: 120,
    status: "Up 2 hours",
    state: "running",
    has_update: false,
    updating: false,
    ports: ["80:80"],
    traefik_url: null,
    registry_url: "https://registry.example.com/nginx",
    ...overrides,
  };
}

export function makeIdleProgress(): BatchProgress {
  return { total: 0, checked: 0, updated: 0, errors: 0, checking: "" };
}

export function makeInspectData(
  overrides: Partial<InspectData> = {},
): InspectData {
  return {
    id: "abc123",
    name: "web",
    image: "nginx:latest",
    created: "2026-01-01T00:00:00Z",
    state: "running",
    status: "Up 2 hours",
    ports: [{ private_port: 80, public_port: 8080, type: "tcp" }],
    mounts: [
      {
        source: "/host/data",
        destination: "/data",
        mode: "rw",
        rw: true,
      },
    ],
    networks: [
      { name: "bridge", ip_address: "172.17.0.2", gateway: "172.17.0.1" },
    ],
    env: ["FOO=bar"],
    labels: { "com.example.role": "web" },
    restart_policy: "unless-stopped",
    health: "healthy",
    ...overrides,
  };
}
