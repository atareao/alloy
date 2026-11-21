/**
 * Query params shared by the paginated REST list (`GET /api/containers`) and
 * the realtime transports (`/api/stream` and `/api/state`), which accept the
 * same filtering/pagination parameters.
 */
export interface ContainerQuery {
  page: number;
  pageSize: number;
  /** Free-text search over name/image/compose_project (server-side). */
  q: string;
  /** Exact state matches (server-side). */
  state: string[];
  /** Restrict to containers with a pending update. */
  hasUpdate: boolean;
}

/** Page sizes offered by the paginator (must match the backend allow-list). */
export const PAGE_SIZE_OPTIONS = [10, 20, 50];

/** Default page size, mirroring the backend default. */
export const DEFAULT_PAGE_SIZE = 20;

/**
 * Serializes a `ContainerQuery` into a query string.
 *
 * `page` and `page_size` are always present; empty filters are omitted so the
 * URL stays canonical and cache-friendly.
 */
export function buildContainerQuery(query: ContainerQuery): string {
  const params = new URLSearchParams();
  params.set("page", String(query.page));
  params.set("page_size", String(query.pageSize));

  const q = query.q.trim();
  if (q) params.set("q", q);
  for (const state of query.state) params.append("state", state);
  if (query.hasUpdate) params.set("has_update", "true");

  return params.toString();
}
