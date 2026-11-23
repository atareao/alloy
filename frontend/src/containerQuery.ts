/**
 * Query params shared by the full REST list (`GET /api/containers`) and the
 * realtime transports (`/api/stream` and `/api/state`), which accept the same
 * filtering parameters. The Dashboard no longer paginates: it requests the
 * whole filtered fleet with `all=true`.
 */
export interface ContainerQuery {
  /** Free-text search over name/image/compose_project (server-side). */
  q: string;
  /** Exact state matches (server-side). */
  state: string[];
  /** Restrict to containers with a pending update. */
  hasUpdate: boolean;
}

/** Page sizes offered by the History paginator (must match the backend allow-list). */
export const PAGE_SIZE_OPTIONS = [10, 20, 50];

/**
 * Serializes a `ContainerQuery` into a query string.
 *
 * Always asks for the complete fleet (`all=true`); empty filters are omitted
 * so the URL stays canonical and cache-friendly.
 */
export function buildContainerQuery(query: ContainerQuery): string {
  const params = new URLSearchParams();
  params.set("all", "true");

  const q = query.q.trim();
  if (q) params.set("q", q);
  for (const state of query.state) params.append("state", state);
  if (query.hasUpdate) params.set("has_update", "true");

  return params.toString();
}
