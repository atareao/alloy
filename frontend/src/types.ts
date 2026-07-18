export interface ContainerInfo {
  id: string;
  name: string;
  image: string;
  image_tag: string;
  size_mb: number;
  status: string;
  state: string;
  has_update: boolean;
  monitored: boolean;
  compose_project?: string;
  ports: string[];
  traefik_url: string | null;
  registry_url: string;
}

export interface UpdateProgress {
  container: string;
  status: string;
  done: boolean;
  error: string | null;
}

export interface NotifEvent {
  container: string;
  status: string;
  timestamp: string;
}

export interface AppConfig {
  oidc_configured: boolean;
  port: number;
  auto_update_enabled: boolean;
  auto_update_interval_hours: number;
  telegram_configured: boolean;
  telegram_token: string | null;
  telegram_chat_id: string | null;
  matrix_configured: boolean;
  matrix_token: string | null;
  matrix_homeserver: string | null;
  matrix_room: string | null;
  webhook_configured: boolean;
  allowed_containers: string[] | null;
}

export interface StackService {
  service: string;
  container_name: string;
  image: string;
  status: string;
  state: string;
}

export interface StackInfo {
  project: string;
  services: StackService[];
}

export interface StackLogEntry {
  service: string;
  container: string;
  lines: string[];
}

export interface StackLogs {
  project: string;
  services: StackLogEntry[];
}

export interface InspectData {
  id: string;
  name: string;
  image: string;
  created: string;
  state: string;
  status: string;
  ports: { private_port: number; public_port: number | null; type: string }[];
  mounts: { source: string; destination: string; mode: string; rw: boolean }[];
  networks: { name: string; ip_address: string; gateway: string }[];
  env: string[];
  labels: Record<string, string>;
  restart_policy: string;
  health: string | null;
}

export interface HistoryEntry {
  container: string;
  image: string;
  old_digest: string;
  new_digest: string;
  timestamp: string;
  status: string;
  duration_ms: number;
}

export type UpdateAction =
  | "none"
  | "pull"
  | "pull-restart"
  | "pull-restart-stack";

export interface UpdatePolicy {
  container: string;
  action: UpdateAction;
  cleanup_old_image: boolean;
  rollback_on_failure: boolean;
}

export interface UpdateCheckConfig {
  cron: string;
  enabled: boolean;
  notify: boolean;
}

export interface DefaultUpdatePolicy {
  action: UpdateAction;
  cleanup_old_image: boolean;
  rollback_on_failure: boolean;
}
