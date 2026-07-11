export function apiFetch(path: string, opts?: RequestInit) {
  // Session cookie is sent automatically
  return fetch(path, {
    ...opts,
    credentials: "include",
    headers: { ...opts?.headers },
  });
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(2) + " " + units[i];
}