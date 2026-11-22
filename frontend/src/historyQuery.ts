import type { HistoryEntry } from "./types";

/**
 * Query params of the paginated history list (`GET /api/history`).
 *
 * Mirrors the backend contract (D1/D2): the page is resolved on the server, so
 * the query always carries `page` and `page_size`; the filters (`q` and the
 * repeatable `status`) are appended only when they have a value.
 */
export interface HistoryQuery {
  page: number;
  pageSize: number;
  /** Free-text search over container/image/digests (server-side). */
  q: string;
  /** Exact status matches (server-side), emitted as repeated `status`. */
  status: string[];
}

/** Page sizes and default, shared with the container list (10/20/50, default 20). */
export { PAGE_SIZE_OPTIONS, DEFAULT_PAGE_SIZE } from "./containerQuery";

/**
 * Serializes a `HistoryQuery` into a query string.
 *
 * `page` and `page_size` are always present; empty filters are omitted so the
 * URL stays canonical (`/api/history?page=1&page_size=20` with no filters).
 */
export function buildHistoryQuery(query: HistoryQuery): string {
  const params = new URLSearchParams();
  params.set("page", String(query.page));
  params.set("page_size", String(query.pageSize));

  const q = query.q.trim();
  if (q) params.set("q", q);
  for (const status of query.status) params.append("status", status);

  return params.toString();
}

/**
 * Stable, data-derived row key (D6): the API exposes no `id`, so the row's
 * position on the page must not be used as a key (React would reuse rows when
 * the page changes). The key is a pure function of the entry's fields.
 */
export function historyRowKey(entry: HistoryEntry): string {
  return `${entry.timestamp}|${entry.container}|${entry.image}|${entry.status}`;
}
