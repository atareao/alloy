export interface ContainerInfo {
  id: string;
  name: string;
  image: string;
  image_tag: string;
  size_mb: number;
  status: string;
  state: string;
  has_update: boolean;
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
  allowed_containers: string[] | null;
}

export interface DockerInfo {
  version: string;
  os: string;
  arch: string;
  containers_total: number;
  containers_running: number;
  containers_paused: number;
  containers_stopped: number;
  images: number;
}

export interface ImageInfo {
  id: string;
  repo: string;
  tag: string;
  repo_tags: string[];
  size_mb: number;
  virtual_size_mb: number;
  created: number;
  containers: number;
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

export interface AlertRule {
  id: string;
  container: string;
  enabled: boolean;
  notify_via: string[];
}

export interface ScheduleEntry {
  id: string;
  container: string;
  target_type: string;
  cron: string;
  action: string;
  enabled: boolean;
  notify: boolean;
  cleanup: string;
}
