use bollard::{
    container::{ListContainersOptions, RestartContainerOptions, StatsOptions},
    system::EventsOptions,
    Docker,
};
use chrono::Local;
use futures::{pin_mut, StreamExt};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::config::Config;
use crate::containers::{fetch_containers, find_container_by_name, pull_image};
use crate::models::*;
use crate::notifications::notify_all;
use crate::state::http_client;
use crate::stats::calc_container_stats;

pub type CachedContainers = Arc<RwLock<Option<Vec<ContainerInfo>>>>;

pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Vec<T> {
    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("load_json: error parsing {}: {}", path, e);
                Vec::new()
            }
        },
        Err(e) => {
            tracing::warn!("load_json: error reading {}: {}", path, e);
            Vec::new()
        }
    }
}

// ── Buffered JSON Writer (P-3) ─────────────────────────────

struct WriteOp {
    path: String,
    data: String,
}

#[derive(Clone)]
pub struct JsonWriter {
    tx: mpsc::UnboundedSender<WriteOp>,
}

static JSON_WRITER: std::sync::OnceLock<JsonWriter> = std::sync::OnceLock::new();

pub fn json_writer() -> &'static JsonWriter {
    JSON_WRITER.get_or_init(JsonWriter::new)
}

impl JsonWriter {
    fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let writer = JsonWriter { tx };
        writer.spawn_flusher(rx);
        writer
    }

    fn spawn_flusher(&self, mut rx: mpsc::UnboundedReceiver<WriteOp>) {
        tokio::spawn(async move {
            let mut buffer: Vec<WriteOp> = Vec::new();
            let mut tick = tokio::time::interval(Duration::from_secs(5));
            loop {
                tokio::select! {
                    op = rx.recv() => {
                        match op {
                            Some(op) => {
                                buffer.push(op);
                                if buffer.len() >= 20 {
                                    flush_buffer(&mut buffer).await;
                                }
                            }
                            None => {
                                flush_buffer(&mut buffer).await;
                                break;
                            }
                        }
                    }
                    _ = tick.tick() => {
                        flush_buffer(&mut buffer).await;
                    }
                }
            }
        });
    }

    pub async fn save<T: serde::Serialize>(&self, path: &str, data: &T) {
        match serde_json::to_string_pretty(data) {
            Ok(json) => {
                let _ = self.tx.send(WriteOp {
                    path: path.to_string(),
                    data: json,
                });
            }
            Err(e) => tracing::warn!("json_writer: error serializing {}: {}", path, e),
        }
    }
}

async fn flush_buffer(buffer: &mut Vec<WriteOp>) {
    if buffer.is_empty() {
        return;
    }
    let mut unique: HashMap<String, String> = HashMap::new();
    for op in buffer.drain(..) {
        unique.insert(op.path, op.data);
    }
    for (path, data) in &unique {
        if let Err(e) = fs::write(path, data) {
            tracing::warn!("json_writer: error writing {}: {}", path, e);
        } else {
            tracing::debug!("json_writer: flushed {} ({})", path, unique.len());
        }
    }
}

pub async fn docker_list_running(docker: &Docker) -> Vec<(String, String, String)> {
    match docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await
    {
        Ok(list) => list
            .iter()
            .filter_map(|c| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n))?;
                let image = c.image.as_deref()?.to_string();
                let id = c.id.as_deref()?.to_string();
                Some((name, image, id))
            })
            .collect(),
        Err(_) => vec![],
    }
}

// ── State Worker: Docker Events API + fallback (P-1 + P-2) ─

pub async fn state_worker(
    docker: Docker,
    config: Config,
    tx: broadcast::Sender<StateEvent>,
    cached_containers: CachedContainers,
) {
    let relevant_actions = [
        "start", "stop", "die", "kill", "pause", "unpause", "restart", "create", "destroy",
        "rename", "update",
    ];

    async fn refresh(
        docker: &Docker,
        config: &Config,
        tx: &broadcast::Sender<StateEvent>,
        cache: &CachedContainers,
    ) {
        let containers = fetch_containers(docker, &config.allowed_containers).await;
        *cache.write().await = Some(containers.clone());
        let _ = tx.send(StateEvent { containers });
    }

    refresh(&docker, &config, &tx, &cached_containers).await;

    loop {
        let options = EventsOptions::<String> {
            since: None,
            until: None,
            filters: HashMap::new(),
        };
        let stream = docker.events(Some(options));
        pin_mut!(stream);
        let mut fallback = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                event = stream.next() => {
                    match event {
                        Some(Ok(evt)) => {
                            if evt.typ == Some(bollard::models::EventMessageTypeEnum::CONTAINER) {
                                if let Some(ref action) = evt.action {
                                    if relevant_actions.contains(&action.as_str()) {
                                        tracing::debug!("Docker event: {} on {:?}", action, evt.actor.as_ref().map(|a| &a.id));
                                        refresh(&docker, &config, &tx, &cached_containers).await;
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            tracing::warn!("Docker events stream error: {} — reconnecting", e);
                            break;
                        }
                        None => {
                            tracing::warn!("Docker events stream ended — reconnecting");
                            break;
                        }
                    }
                }
                _ = fallback.tick() => {
                    refresh(&docker, &config, &tx, &cached_containers).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

pub async fn auto_update_worker(
    docker: Docker,
    config: Config,
    notif_tx: broadcast::Sender<NotifEvent>,
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
) {
    if !config.auto_update() {
        return;
    }
    let hours = config.auto_update_interval();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(hours * 3600));
    loop {
        interval.tick().await;
        for (name, image, cid) in docker_list_running(&docker).await {
            let start_time = std::time::Instant::now();
            if !pull_image(&docker, &image).await {
                continue;
            }
            if docker
                .restart_container(&cid, None::<RestartContainerOptions>)
                .await
                .is_ok()
            {
                let _ = notif_tx.send(NotifEvent {
                    container: name.clone(),
                    status: "🤖 auto-updated".into(),
                    timestamp: Local::now().format("%H:%M:%S").to_string(),
                });
                notify_all(&config, &name, "🤖 auto-actualizado").await;
                let entry = UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.clone(),
                    old_digest: String::new(),
                    new_digest: String::new(),
                    timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                    status: "auto-update".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                };
                let mut hist = update_history.lock().await;
                hist.push(entry);
                json_writer().save(FILE_UPDATES_HISTORY, &*hist).await;
            }
        }
    }
}

pub async fn alerts_worker(
    docker: Docker,
    config: Config,
    notif_tx: broadcast::Sender<NotifEvent>,
    alerts: Arc<Mutex<Vec<AlertConfig>>>,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(30));
    loop {
        tick.tick().await;
        let alerts_list = alerts.lock().await.clone();
        let containers = docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();
        let container_map: HashMap<String, &bollard::models::ContainerSummary> = containers
            .iter()
            .filter_map(|c| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n))?;
                Some((name, c))
            })
            .collect();
        for alert in &alerts_list {
            if !alert.enabled {
                continue;
            }
            let container_name = &alert.container;
            let Some(container) = container_map.get(container_name) else {
                continue;
            };
            let cid = match container.id.as_deref() {
                Some(id) => id,
                None => continue,
            };
            match alert.r#type.as_str() {
                "status" => {
                    let container_state = container.state.as_deref().unwrap_or("unknown");
                    if container_state == "exited"
                        || container_state == "dead"
                        || container_state == "paused"
                    {
                        let msg = format!(
                            "⚠️ Container '{}' está en estado: {}",
                            container_name, container_state
                        );
                        let _ = notif_tx.send(NotifEvent {
                            container: container_name.clone(),
                            status: format!("alert: {}", container_state),
                            timestamp: Local::now().format("%H:%M:%S").to_string(),
                        });
                        if alert.notify_via.contains(&"telegram".to_string())
                            || alert.notify_via.contains(&"matrix".to_string())
                        {
                            notify_all(&config, container_name, &msg).await;
                        }
                    }
                }
                "cpu" | "memory" => {
                    let mut stats_stream = docker.stats(cid, None::<StatsOptions>);
                    if let Some(Ok(stats)) = stats_stream.next().await {
                        let container_stats = calc_container_stats(container_name, &stats);
                        let value = if alert.r#type == "cpu" {
                            container_stats.cpu_percent
                        } else {
                            if container_stats.memory_limit_mb > 0.0 {
                                (container_stats.memory_usage_mb / container_stats.memory_limit_mb)
                                    * 100.0
                            } else {
                                0.0
                            }
                        };
                        if value > alert.threshold {
                            let msg = format!(
                                "⚠️ '{}' {} está en {:.1}% (umbral: {}%)",
                                container_name, alert.r#type, value, alert.threshold
                            );
                            let _ = notif_tx.send(NotifEvent {
                                container: container_name.clone(),
                                status: format!("alert: {}={:.1}%", alert.r#type, value),
                                timestamp: Local::now().format("%H:%M:%S").to_string(),
                            });
                            if alert.notify_via.contains(&"telegram".to_string())
                                || alert.notify_via.contains(&"matrix".to_string())
                            {
                                notify_all(&config, container_name, &msg).await;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub async fn health_checks_worker(
    _docker: Docker,
    _config: Config,
    notif_tx: broadcast::Sender<NotifEvent>,
    health_checks: Arc<Mutex<Vec<HealthCheck>>>,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(30));
    loop {
        tick.tick().await;
        let hcs = health_checks.lock().await.clone();
        for mut hc in hcs {
            if !hc.enabled {
                continue;
            }
            let should_run = match &hc.last_result {
                Some(last) => {
                    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(
                        &last.last_checked,
                        "%Y-%m-%dT%H:%M:%S",
                    ) {
                        let elapsed = (Local::now().naive_local() - parsed).num_seconds() as u64;
                        elapsed >= hc.interval_secs
                    } else {
                        true
                    }
                }
                None => true,
            };
            if !should_run {
                continue;
            }
            let start = std::time::Instant::now();
            let (status, latency) = match hc.r#type.as_str() {
                "http" => {
                    match http_client()
                        .get(&hc.target)
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                ("healthy".to_string(), start.elapsed().as_millis() as u64)
                            } else {
                                ("unhealthy".to_string(), start.elapsed().as_millis() as u64)
                            }
                        }
                        Err(_) => ("unhealthy".to_string(), start.elapsed().as_millis() as u64),
                    }
                }
                "ping" => {
                    match tokio::process::Command::new("ping")
                        .args(["-c", "1", "-W", "2", &hc.target])
                        .output()
                        .await
                    {
                        Ok(output) => {
                            if output.status.success() {
                                ("healthy".to_string(), start.elapsed().as_millis() as u64)
                            } else {
                                ("unhealthy".to_string(), start.elapsed().as_millis() as u64)
                            }
                        }
                        Err(_) => ("unhealthy".to_string(), start.elapsed().as_millis() as u64),
                    }
                }
                _ => ("unknown".to_string(), 0),
            };
            hc.last_result = Some(HealthCheckResult {
                target: hc.target.clone(),
                status: status.clone(),
                latency_ms: latency,
                last_checked: Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            });
            let mut hcs_lock = health_checks.lock().await;
            if let Some(existing) = hcs_lock.iter_mut().find(|h| h.id == hc.id) {
                existing.last_result = hc.last_result.clone();
            }
            json_writer().save("health_checks.json", &*hcs_lock).await;
            if status == "unhealthy" {
                let _ = notif_tx.send(NotifEvent {
                    container: hc.container.clone().unwrap_or_default(),
                    status: format!("health-check FAIL: {}", hc.target),
                    timestamp: Local::now().format("%H:%M:%S").to_string(),
                });
            }
        }
    }
}

pub async fn scheduler_worker(
    docker: Docker,
    _config: Config,
    update_tx: broadcast::Sender<UpdateProgress>,
    notif_tx: broadcast::Sender<NotifEvent>,
    schedules: Arc<Mutex<Vec<ScheduleTask>>>,
) {
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        tick.tick().await;
        let now = Local::now();
        let tasks = schedules.lock().await.clone();
        for task in &tasks {
            if !task.enabled {
                continue;
            }
            if !match_cron(&task.cron, &now) {
                continue;
            }
            tracing::info!(
                "Scheduler: ejecutando '{}' en container '{}'",
                task.action,
                task.container
            );
            match task.action.as_str() {
                "update" | "restart" => {
                    if let Ok(container) = find_container_by_name(&docker, &task.container).await {
                        if let Some(cid) = container.id.as_deref() {
                            if task.action == "update" {
                                if let Some(image) = container.image.as_deref() {
                                    let _ = update_tx.send(UpdateProgress {
                                        container: task.container.clone(),
                                        status: format!("[scheduled] pulling {}", image),
                                        done: false,
                                        error: None,
                                    });
                                    if pull_image(&docker, image).await {
                                        let _ = docker
                                            .restart_container(cid, None::<RestartContainerOptions>)
                                            .await;
                                    }
                                }
                            } else {
                                let _ = docker
                                    .restart_container(cid, None::<RestartContainerOptions>)
                                    .await;
                            }
                            let _ = notif_tx.send(NotifEvent {
                                container: task.container.clone(),
                                status: format!("🕐 scheduled {}", task.action),
                                timestamp: Local::now().format("%H:%M:%S").to_string(),
                            });
                        }
                    }
                }
                _ => {
                    tracing::warn!("Scheduler: acción desconocida '{}'", task.action);
                }
            }
        }
    }
}

fn match_cron(cron: &str, dt: &chrono::DateTime<Local>) -> bool {
    let expr = format!("0 {}", cron);
    match expr.parse::<cron::Schedule>() {
        Ok(schedule) => schedule.includes(dt.to_utc()),
        Err(e) => {
            tracing::warn!("Invalid cron expression '{}': {}", cron, e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    #[test]
    fn test_load_json_file_not_found() {
        let result: Vec<UpdateHistoryEntry> = load_json("/tmp/nonexistent_file_xyz.json");
        assert!(result.is_empty());
    }

    #[test]
    fn test_load_json_valid_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.json");
        let data = r#"[
            {"container": "web", "image": "nginx", "old_digest": "abc", "new_digest": "def",
             "timestamp": "2024-01-01T00:00:00", "status": "ok", "duration_ms": 100}
        ]"#;
        std::fs::write(&path, data).unwrap();
        let result: Vec<UpdateHistoryEntry> = load_json(path.to_str().unwrap());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].container, "web");
    }

    #[test]
    fn test_load_json_invalid_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not valid json").unwrap();
        let result: Vec<UpdateHistoryEntry> = load_json(path.to_str().unwrap());
        assert!(result.is_empty());
    }

    #[test]
    fn test_load_json_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty.json");
        std::fs::write(&path, "[]").unwrap();
        let result: Vec<UpdateHistoryEntry> = load_json(path.to_str().unwrap());
        assert!(result.is_empty());
    }

    #[test]
    fn test_match_cron_every_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("* * * * *", &dt));
    }

    #[test]
    fn test_match_cron_specific_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 30, 0).unwrap();
        assert!(match_cron("30 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_wrong_minute() {
        // Create a time with minute that won't match "0"
        // "0 * * * *" means minute 0 of every hour
        let dt = Local::now();
        let minute: u32 = dt.format("%M").to_string().parse().unwrap_or(99);
        if minute == 0 {
            // Can't test this case if we're at minute 0
            return;
        }
        assert!(!match_cron("0 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_invalid_expression() {
        let dt = Local::now();
        assert!(!match_cron("invalid", &dt));
    }

    #[test]
    fn test_match_cron_empty() {
        let dt = Local::now();
        assert!(!match_cron("", &dt));
    }
}
