use bollard::Docker;
use chrono::Local;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::models::*;
use crate::notifications::notify_selected;

/// Monitoriza cambios de estado de los contenedores y notifica.
/// Solo notifica transiciones: running → algo (problema) y algo → running (recuperación).
pub async fn alerts_worker(
    docker: Docker,
    config: Config,
    settings: Arc<Mutex<Settings>>,
    notif_tx: broadcast::Sender<NotifEvent>,
    alerts: Arc<Mutex<Vec<AlertConfig>>>,
) {
    let mut previous_states: HashMap<String, String> = HashMap::new();
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(15));

    loop {
        tick.tick().await;
        let alerts_list = alerts.lock().await.clone();
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

        for alert in &alerts_list {
            if !alert.enabled {
                continue;
            }
            let container_name = &alert.container;
            let Some(container) = container_map.get(container_name) else {
                let prev = previous_states.insert(container_name.clone(), "gone".into());
                if prev.as_deref() != Some("gone") {
                    let msg = format!("⚠️ Container '{}' ha desaparecido", container_name);
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: "alert: gone".into(),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_selected(&config, &settings, container_name, &msg, &alert.notify_via)
                        .await;
                }
                continue;
            };
            let current_state = container.state.as_deref().unwrap_or("unknown").to_string();
            let prev_state = previous_states.insert(container_name.clone(), current_state.clone());

            if let Some(prev) = prev_state {
                if prev == "running"
                    && (current_state == "exited"
                        || current_state == "dead"
                        || current_state == "paused"
                        || current_state == "restarting")
                {
                    let msg = format!(
                        "⚠️ Container '{}' ha cambiado a: {}",
                        container_name, current_state
                    );
                    let _ = notif_tx.send(NotifEvent {
                        container: container_name.clone(),
                        status: format!("alert: {}", current_state),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                    });
                    notify_selected(&config, &settings, container_name, &msg, &alert.notify_via)
                        .await;
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
                    notify_selected(&config, &settings, container_name, &msg, &alert.notify_via)
                        .await;
                }
            }
        }
    }
}
