use bollard::Docker;
use chrono::Local;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::models::*;
use crate::notifications::notify_all;

/// Monitoriza cambios de estado de los contenedores y notifica.
/// Solo notifica transiciones: running → algo (problema) y algo → running (recuperación).
/// Solo monitoriza contenedores en `settings.monitored_containers`.
pub async fn alerts_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    notif_tx: broadcast::Sender<NotifEvent>,
) {
    let mut previous_states: HashMap<String, String> = HashMap::new();
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(15));

    loop {
        tick.tick().await;
        let monitored = settings.lock().await.monitored_containers.clone();
        let containers = docker
            .list_containers(Some(bollard::container::ListContainersOptions::<String> {
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

        tracing::debug!(
            "alerts_worker: checking {} monitored containers",
            monitored.len()
        );
        for container_name in &monitored {
            let Some(container) = container_map.get(container_name) else {
                let prev = previous_states.insert(container_name.clone(), "gone".into());
                if prev.as_deref() != Some("gone") {
                    let msg = format!("⚠️ Container '{}' ha desaparecido", container_name);
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: "alert: gone".into(),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_all(&config, &settings, container_name, &msg).await;
                }
                continue;
            };
            let current_state = container.state.as_deref().unwrap_or("unknown").to_string();
            let prev_state = previous_states.insert(container_name.clone(), current_state.clone());
            tracing::debug!(
                "alerts_worker: {} state={:?} prev={:?}",
                container_name,
                current_state,
                prev_state
            );

            if let Some(prev) = prev_state {
                if prev == "running"
                    && (current_state == "exited"
                        || current_state == "dead"
                        || current_state == "paused"
                        || current_state == "restarting")
                {
                    tracing::info!(
                        "alerts_worker: 🔔 {}: {} → {}",
                        container_name,
                        prev,
                        current_state,
                    );
                    let msg = format!(
                        "⚠️ Container '{}' ha cambiado a: {}",
                        container_name, current_state
                    );
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: format!("alert: {}", current_state),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_all(&config, &settings, container_name, &msg).await;
                }
                if current_state == "running"
                    && (prev == "exited"
                        || prev == "dead"
                        || prev == "paused"
                        || prev == "restarting")
                {
                    let msg = format!("✅ Container '{}' ha vuelto a running", container_name);
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: "alert: recovered".into(),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_all(&config, &settings, container_name, &msg).await;
                }
            }
        }
    }
}
