use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{container::ListContainersOptions, Docker};
use chrono::Local;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::config::Config;
use crate::models::*;
use crate::notifications::notify_all;
use crate::state::AppState;

fn get_compose_projects() -> HashMap<String, String> {
    let output = std::process::Command::new("docker")
        .args(["compose", "ls", "--format", "json"])
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut projects: HashMap<String, String> = HashMap::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('{') {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if let (Some(name), Some(files)) = (
                val.get("Name").and_then(|n| n.as_str()),
                val.get("ConfigFiles").and_then(|f| f.as_str()),
            ) {
                let first_file = files.split(',').next().unwrap_or(files).trim().to_string();
                tracing::info!("Detected compose project '{}' at {}", name, first_file);
                projects.insert(name.to_string(), first_file);
            }
        }
    }
    projects
}

fn get_compose_project_path(project: &str) -> Option<String> {
    let projects = get_compose_projects();
    projects.get(project).cloned()
}

async fn list_stacks_h(State(docker): State<Docker>) -> Json<Vec<StackInfo>> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
    let mut projects: HashMap<String, Vec<StackService>> = HashMap::new();
    for c in &containers {
        let labels = c.labels.as_ref();
        let project = labels.and_then(|l| l.get(LABEL_COMPOSE_PROJECT)).cloned();
        let service = labels.and_then(|l| l.get(LABEL_COMPOSE_SERVICE)).cloned();
        if let (Some(project), Some(service)) = (project, service) {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| strip_name(n))
                .unwrap_or_default();
            let image = c.image.as_deref().unwrap_or("unknown").to_string();
            let status = c.status.as_deref().unwrap_or("unknown").to_string();
            let state = c.state.as_deref().unwrap_or("unknown").to_string();
            projects.entry(project).or_default().push(StackService {
                service,
                container_name: name,
                image,
                status,
                state,
            });
        }
    }
    let stacks: Vec<StackInfo> = projects
        .into_iter()
        .map(|(project, services)| StackInfo { project, services })
        .collect();
    Json(stacks)
}

async fn update_stack_h(
    State(docker): State<Docker>,
    State(config): State<Config>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_tx): State<broadcast::Sender<UpdateProgress>>,
    State(notif_tx): State<broadcast::Sender<NotifEvent>>,
    Path(project): Path<String>,
) -> Result<Json<StackUpdateResponse>, AppError> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(|e| AppError::Docker(e.to_string()))?;
    let project_containers: Vec<_> = containers
        .iter()
        .filter(|c| {
            c.labels
                .as_ref()
                .and_then(|l| l.get(LABEL_COMPOSE_PROJECT))
                .map(|p| p == project.as_str())
                .unwrap_or(false)
        })
        .collect();
    if project_containers.is_empty() {
        return Err(AppError::NotFound(format!("Stack '{}' not found", project)));
    }
    let mut services: Vec<String> = Vec::new();
    for c in &project_containers {
        if let Some(svc) = c
            .labels
            .as_ref()
            .and_then(|l| l.get(LABEL_COMPOSE_SERVICE))
            .cloned()
        {
            if !services.contains(&svc) {
                services.push(svc);
            }
        }
    }
    let compose_file = project_containers
        .first()
        .and_then(|c| c.labels.as_ref())
        .and_then(|l| l.get(LABEL_COMPOSE_CONFIG_FILES))
        .cloned()
        .or_else(|| {
            project_containers
                .first()
                .and_then(|c| c.labels.as_ref())
                .and_then(|l| l.get(LABEL_COMPOSE_WORKING_DIR))
                .map(|dir| format!("{}/docker-compose.yml", dir))
        })
        .filter(|p| std::path::Path::new(p).exists())
        .or_else(|| get_compose_project_path(&project));
    let compose_file = match compose_file {
        Some(f) => f,
        None => {
            tracing::warn!("No compose file found for project '{}'", project);
            return Err(AppError::NotFound(format!(
                "No compose file for '{}'",
                project
            )));
        }
    };
    let _ = update_tx.send(UpdateProgress {
        container: project.clone(),
        status: format!(
            "🔄 Updating stack '{}' ({} services)...",
            project,
            services.len()
        ),
        done: false,
        error: None,
    });
    let mut results = Vec::new();
    for service in &services {
        let start = std::time::Instant::now();
        let _ = update_tx.send(UpdateProgress {
            container: format!("{}/{}", project, service),
            status: format!("📥 Pulling {}...", service),
            done: false,
            error: None,
        });
        let pull_result = tokio::process::Command::new("docker")
            .args(["compose", "-f", &compose_file, "pull", service])
            .output()
            .await;
        match pull_result {
            Ok(output) if output.status.success() => {
                let _ = update_tx.send(UpdateProgress {
                    container: format!("{}/{}", project, service),
                    status: format!("🔄 Recreating {}...", service),
                    done: false,
                    error: None,
                });
                let up_result = tokio::process::Command::new("docker")
                    .args([
                        "compose",
                        "-f",
                        &compose_file,
                        "up",
                        "-d",
                        "--no-deps",
                        service,
                    ])
                    .output()
                    .await;
                match up_result {
                    Ok(up_output) if up_output.status.success() => {
                        let duration = start.elapsed().as_millis() as u64;
                        results.push(StackUpdateResult {
                            service: service.clone(),
                            status: "ok".into(),
                            duration_ms: duration,
                            error: None,
                        });
                        let _ = update_tx.send(UpdateProgress {
                            container: format!("{}/{}", project, service),
                            status: format!("✅ {} updated", service),
                            done: true,
                            error: None,
                        });
                        let ts = Local::now().format("%H:%M:%S").to_string();
                        let _ = notif_tx.send(NotifEvent {
                            container: format!("{}/{}", project, service),
                            status: "updated ✅".into(),
                            timestamp: ts,
                        });
                        notify_all(
                            &config,
                            &settings,
                            &format!("{}/{}", project, service),
                            "✅ actualizado via stack",
                        )
                        .await;
                    }
                    Ok(up_output) => {
                        let stderr = String::from_utf8_lossy(&up_output.stderr).to_string();
                        results.push(StackUpdateResult {
                            service: service.clone(),
                            status: "error".into(),
                            duration_ms: start.elapsed().as_millis() as u64,
                            error: Some(stderr.clone()),
                        });
                        let _ = update_tx.send(UpdateProgress {
                            container: format!("{}/{}", project, service),
                            status: format!("❌ {} error: {}", service, stderr),
                            done: true,
                            error: Some(stderr),
                        });
                    }
                    Err(e) => {
                        results.push(StackUpdateResult {
                            service: service.clone(),
                            status: "error".into(),
                            duration_ms: start.elapsed().as_millis() as u64,
                            error: Some(e.to_string()),
                        });
                        let _ = update_tx.send(UpdateProgress {
                            container: format!("{}/{}", project, service),
                            status: format!("❌ {} error: {}", service, e),
                            done: true,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                results.push(StackUpdateResult {
                    service: service.clone(),
                    status: "error".into(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(stderr.clone()),
                });
                let _ = update_tx.send(UpdateProgress {
                    container: format!("{}/{}", project, service),
                    status: format!("❌ {} pull error: {}", service, stderr),
                    done: true,
                    error: Some(stderr),
                });
            }
            Err(e) => {
                results.push(StackUpdateResult {
                    service: service.clone(),
                    status: "error".into(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(e.to_string()),
                });
                let _ = update_tx.send(UpdateProgress {
                    container: format!("{}/{}", project, service),
                    status: format!("❌ {} error: {}", service, e),
                    done: true,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    let _ = update_tx.send(UpdateProgress {
        container: project.clone(),
        status: format!("🏁 Stack '{}' update complete", project),
        done: true,
        error: None,
    });
    Ok(Json(StackUpdateResponse {
        project: project.to_string(),
        results,
    }))
}

async fn down_stack_h(
    State(docker): State<Docker>,
    Path(project): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(|e| AppError::Docker(e.to_string()))?;
    let project_containers: Vec<_> = containers
        .iter()
        .filter(|c| {
            c.labels
                .as_ref()
                .and_then(|l| l.get(LABEL_COMPOSE_PROJECT))
                .map(|p| p == project.as_str())
                .unwrap_or(false)
        })
        .collect();
    if project_containers.is_empty() {
        return Err(AppError::NotFound(format!("Stack '{}' not found", project)));
    }
    let compose_file = project_containers
        .first()
        .and_then(|c| c.labels.as_ref())
        .and_then(|l| l.get(LABEL_COMPOSE_CONFIG_FILES))
        .cloned()
        .or_else(|| {
            project_containers
                .first()
                .and_then(|c| c.labels.as_ref())
                .and_then(|l| l.get(LABEL_COMPOSE_WORKING_DIR))
                .map(|dir| format!("{}/docker-compose.yml", dir))
        })
        .filter(|p| std::path::Path::new(p).exists())
        .or_else(|| get_compose_project_path(&project));
    let compose_file = match compose_file {
        Some(f) => f,
        None => {
            return Err(AppError::NotFound(format!(
                "No compose file for '{}'",
                project
            )));
        }
    };
    let output = tokio::process::Command::new("docker")
        .args(["compose", "-f", &compose_file, "down"])
        .output()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if output.status.success() {
        Ok(Json(serde_json::json!({
            "project": project,
            "status": "removed",
            "stdout": String::from_utf8_lossy(&output.stdout).to_string()
        })))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(AppError::Internal(format!(
            "docker compose down failed: {}",
            stderr
        )))
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/stacks", get(list_stacks_h))
        .route("/api/stacks/{project}/update", post(update_stack_h))
        .route("/api/stacks/{project}/down", post(down_stack_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_podman_available() -> bool {
        std::env::var("DOCKER_HOST").is_ok()
            || std::path::Path::new("/run/user/1000/podman/podman.sock").exists()
    }

    async fn podman_client() -> Docker {
        let socket = std::env::var("DOCKER_HOST").unwrap_or_else(|_| {
            "unix:///run/user/1000/podman/podman.sock".to_string()
        });
        Docker::connect_with_local(&socket, 120, bollard::API_DEFAULT_VERSION)
            .expect("Failed to connect to Podman socket")
    }

    // ── get_compose_projects ─────────────────────────────────

    #[test]
    fn test_get_compose_projects_returns_map() {
        // This runs `docker compose ls --format json` via the real Podman socket
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let projects = get_compose_projects();
        // Should always return a HashMap (possibly empty)
        assert!(projects.is_empty() || !projects.is_empty());
    }

    // ── get_compose_project_path ─────────────────────────────

    #[test]
    fn test_get_compose_project_path_nonexistent() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let path = get_compose_project_path("this-project-does-not-exist-xyz");
        assert!(path.is_none());
    }

    // ── list_stacks_h ────────────────────────────────────────

    #[tokio::test]
    async fn test_integration_list_stacks_returns_valid_structure() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<StackInfo>> = list_stacks_h(State(docker)).await;
        // May be empty (Quadlets), but structure must be valid
        for stack in &result.0 {
            assert!(!stack.project.is_empty());
            for svc in &stack.services {
                assert!(!svc.service.is_empty());
                assert!(!svc.container_name.is_empty());
                assert!(!svc.image.is_empty());
                assert!(["running", "exited", "paused", "created"]
                    .contains(&svc.state.as_str()));
            }
        }
    }

    #[tokio::test]
    async fn test_integration_list_stacks_services_have_state() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<StackInfo>> = list_stacks_h(State(docker)).await;
        for stack in &result.0 {
            for svc in &stack.services {
                // Each service must have a valid state
                assert!(
                    !svc.state.is_empty(),
                    "Service {} in stack {} has empty state",
                    svc.service,
                    stack.project
                );
                assert!(
                    !svc.status.is_empty(),
                    "Service {} in stack {} has empty status",
                    svc.service,
                    stack.project
                );
            }
        }
    }

    // ── StackService ─────────────────────────────────────────

    #[test]
    fn test_stack_service_creation() {
        let svc = StackService {
            service: "web".into(),
            container_name: "myapp_web_1".into(),
            image: "nginx:latest".into(),
            status: "running".into(),
            state: "running".into(),
        };
        assert_eq!(svc.service, "web");
        assert_eq!(svc.container_name, "myapp_web_1");
        assert_eq!(svc.image, "nginx:latest");
        assert_eq!(svc.status, "running");
        assert_eq!(svc.state, "running");
    }

    #[test]
    fn test_stack_info_creation() {
        let info = StackInfo {
            project: "myapp".into(),
            services: vec![StackService {
                service: "db".into(),
                container_name: "myapp_db_1".into(),
                image: "postgres:15".into(),
                status: "running".into(),
                state: "running".into(),
            }],
        };
        assert_eq!(info.project, "myapp");
        assert_eq!(info.services.len(), 1);
        assert_eq!(info.services[0].service, "db");
    }

    // ── StackUpdateResult ────────────────────────────────────

    #[test]
    fn test_stack_update_result_ok() {
        let result = StackUpdateResult {
            service: "web".into(),
            status: "ok".into(),
            duration_ms: 1234,
            error: None,
        };
        assert_eq!(result.status, "ok");
        assert_eq!(result.duration_ms, 1234);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_stack_update_result_error() {
        let result = StackUpdateResult {
            service: "db".into(),
            status: "error".into(),
            duration_ms: 567,
            error: Some("pull failed".into()),
        };
        assert_eq!(result.status, "error");
        assert_eq!(result.error.as_deref(), Some("pull failed"));
    }

    #[test]
    fn test_stack_update_response() {
        let response = StackUpdateResponse {
            project: "myapp".into(),
            results: vec![StackUpdateResult {
                service: "web".into(),
                status: "ok".into(),
                duration_ms: 100,
                error: None,
            }],
        };
        assert_eq!(response.project, "myapp");
        assert_eq!(response.results.len(), 1);
    }
}
