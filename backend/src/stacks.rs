use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
        StopContainerOptions,
    },
    network::ListNetworksOptions,
    Docker,
};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::{self, DbPool};
use crate::models::*;
use crate::notifications::notify_all;
use crate::state::AppState;

/// Container identity used by the API-driven `down` planner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownTarget {
    pub id: String,
    pub name: String,
}

/// Pure: select the containers that belong to `project` (compose label).
pub(crate) fn select_project_containers<'a>(
    containers: &'a [bollard::models::ContainerSummary],
    project: &str,
) -> Vec<&'a bollard::models::ContainerSummary> {
    containers
        .iter()
        .filter(|c| {
            c.labels
                .as_ref()
                .and_then(|l| l.get(LABEL_COMPOSE_PROJECT))
                .map(|p| p == project)
                .unwrap_or(false)
        })
        .collect()
}

/// Pure: ordered stop+remove plan for a project's containers.
pub(crate) fn build_down_plan(
    containers: &[bollard::models::ContainerSummary],
    project: &str,
) -> Vec<DownTarget> {
    select_project_containers(containers, project)
        .into_iter()
        .filter_map(|c| {
            Some(DownTarget {
                id: c.id.clone()?,
                name: c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n))
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Pure: the networks that belong to `project` (compose label).
pub(crate) fn project_networks(
    networks: &[bollard::models::Network],
    project: &str,
) -> Vec<bollard::models::Network> {
    networks
        .iter()
        .filter(|n| {
            n.labels
                .as_ref()
                .and_then(|l| l.get(LABEL_COMPOSE_PROJECT))
                .map(|p| p == project)
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// Pure: unique compose service names for a project, in encounter order.
pub(crate) fn project_services(
    containers: &[bollard::models::ContainerSummary],
    project: &str,
) -> Vec<String> {
    let mut services: Vec<String> = Vec::new();
    for c in select_project_containers(containers, project) {
        if let Some(svc) = c.labels.as_ref().and_then(|l| l.get(LABEL_COMPOSE_SERVICE)) {
            if !services.contains(svc) {
                services.push(svc.clone());
            }
        }
    }
    services
}

/// Pure: build the update-history entry recorded after a successful
/// stack-service recreate.
pub(crate) fn stack_history_entry(
    name: &str,
    image: &str,
    old_digest: &str,
    new_digest: &str,
    duration_ms: u64,
) -> UpdateHistoryEntry {
    UpdateHistoryEntry {
        container: name.to_string(),
        image: image.to_string(),
        old_digest: old_digest.to_string(),
        new_digest: new_digest.to_string(),
        timestamp: crate::timezone::now_formatted(),
        status: "stack-update".into(),
        duration_ms,
    }
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
    State(settings): State<Arc<Mutex<Settings>>>,
    State(db_pool): State<DbPool>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    Path(project): Path<String>,
) -> Result<Json<StackUpdateResponse>, AppError> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .map_err(|e| AppError::Docker(e.to_string()))?;
    let project_members = select_project_containers(&containers, &project);
    if project_members.is_empty() {
        return Err(AppError::NotFound(format!("Stack '{}' not found", project)));
    }
    let services = project_services(&containers, &project);
    let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
    let mut results = Vec::new();
    for service in &services {
        let start = std::time::Instant::now();
        let members: Vec<_> = project_members
            .iter()
            .filter(|c| {
                c.labels
                    .as_ref()
                    .and_then(|l| l.get(LABEL_COMPOSE_SERVICE))
                    .map(|s| s == service)
                    .unwrap_or(false)
            })
            .collect();
        let mut error: Option<String> = None;
        for c in members {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| strip_name(n))
                .unwrap_or_default();
            let image = c.image.clone().unwrap_or_default();
            let cid = c.id.clone().unwrap_or_default();
            if cid.is_empty() || image.is_empty() {
                error = Some(format!("contenedor '{}' sin id/imagen", name));
                continue;
            }
            let old_digest = c.image_id.clone().unwrap_or_default();
            let c_start = std::time::Instant::now();
            // Pull + recreate via the Docker API (no `docker compose` CLI).
            match crate::updates::handlers::pull_and_recreate(
                &docker,
                &name,
                &cid,
                &image,
                None,
                pull_timeout,
            )
            .await
            {
                Ok(()) => {
                    // Persist the config digest of the freshly deployed service
                    // so the next update check compares against the image
                    // actually running (parity with the scheduler path).
                    let new_digest = persist_stack_digest(&docker, &db_pool, &name).await;
                    // The container now runs the freshly pulled image.
                    clear_has_update(&db_pool, &name).await;
                    let entry = stack_history_entry(
                        &name,
                        &image,
                        &old_digest,
                        &new_digest,
                        c_start.elapsed().as_millis() as u64,
                    );
                    append_stack_history(&db_pool, &update_history, entry).await;
                }
                Err(e) => {
                    tracing::error!(
                        "update_stack_h: servicio '{}' contenedor '{}': {}",
                        service,
                        name,
                        e
                    );
                    error = Some(e);
                }
            }
        }
        match error {
            None => {
                results.push(StackUpdateResult {
                    service: service.clone(),
                    status: "ok".into(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                });
                notify_all(
                    &settings,
                    &format!("{}/{}", project, service),
                    "✅ actualizado via stack",
                )
                .await;
            }
            Some(e) => {
                results.push(StackUpdateResult {
                    service: service.clone(),
                    status: "error".into(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(e),
                });
            }
        }
    }
    Ok(Json(StackUpdateResponse {
        project: project.to_string(),
        results,
    }))
}

/// Inspect the recreated container and persist its config digest as
/// `last_remote_digest` (parity with the scheduler path). Returns the digest
/// that was persisted, or an empty string when unavailable.
async fn persist_stack_digest(docker: &Docker, db_pool: &DbPool, name: &str) -> String {
    let inspect = match docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
    {
        Ok(i) => i,
        Err(_) => return String::new(),
    };
    let Some(image_id) = inspect.image else {
        return String::new();
    };
    match db_pool.get().await {
        Ok(conn) => {
            if let Ok(guard) = conn.lock() {
                let _ = db::update_container_last_remote_digest(&guard, name, &image_id);
            } else {
                tracing::error!(
                    "update_stack_h: mutex de DB poisoned al persistir digest de '{}'",
                    name
                );
            }
        }
        Err(_) => tracing::error!(
            "update_stack_h: no se pudo obtener conexión DB para '{}'",
            name
        ),
    }
    image_id
}

/// Clear `has_update` for a container after a successful stack recreate.
async fn clear_has_update(db_pool: &DbPool, name: &str) {
    match db_pool.get().await {
        Ok(conn) => {
            if let Ok(guard) = conn.lock() {
                let _ = db::update_container_has_update(&guard, name, false);
            } else {
                tracing::error!(
                    "update_stack_h: mutex de DB poisoned al limpiar has_update de '{}'",
                    name
                );
            }
        }
        Err(_) => tracing::error!(
            "update_stack_h: no se pudo obtener conexión DB para '{}'",
            name
        ),
    }
}

/// Append a stack-update entry to the in-memory history and the DB.
async fn append_stack_history(
    db_pool: &DbPool,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    entry: UpdateHistoryEntry,
) {
    let mut hist = update_history.lock().await;
    hist.push(entry);
    if let Ok(conn) = db_pool.get().await {
        match conn.lock() {
            Ok(guard) => {
                let _ = db::append_update_history(&guard, hist.last().unwrap());
            }
            Err(_) => tracing::error!(
                "update_stack_h: mutex de DB poisoned al persistir historial de '{}'",
                hist.last().map(|e| e.container.as_str()).unwrap_or("")
            ),
        }
    }
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
    let plan = build_down_plan(&containers, &project);
    if plan.is_empty() {
        return Err(AppError::NotFound(format!("Stack '{}' not found", project)));
    }
    // Stop + remove each container of the project via the Docker API.
    let mut failures: Vec<String> = Vec::new();
    for target in &plan {
        if let Err(e) = docker
            .stop_container(&target.id, None::<StopContainerOptions>)
            .await
        {
            // Already-stopped containers return 304; tolerate and continue.
            tracing::warn!("down_stack_h: stop de '{}' falló: {}", target.name, e);
        }
        if let Err(e) = docker
            .remove_container(&target.id, None::<RemoveContainerOptions>)
            .await
        {
            let msg = format!("remove de '{}' falló: {}", target.name, e);
            tracing::error!("down_stack_h: {}", msg);
            failures.push(msg);
        }
    }
    // Remove the project networks. In-use networks are tolerated with a WARN.
    let filter_label = format!("{}={}", LABEL_COMPOSE_PROJECT, project);
    let networks = docker
        .list_networks(Some(ListNetworksOptions::<String> {
            filters: HashMap::from([("label".to_string(), vec![filter_label])]),
        }))
        .await
        .unwrap_or_default();
    for network in project_networks(&networks, &project) {
        let Some(id) = network.id else {
            continue;
        };
        if let Err(e) = docker.remove_network(&id).await {
            tracing::warn!(
                "down_stack_h: red '{}' en uso o no eliminable, se continúa: {}",
                id,
                e
            );
        }
    }
    if !failures.is_empty() {
        return Err(AppError::Internal(format!(
            "down de '{}' incompleto: {}",
            project,
            failures.join("; ")
        )));
    }
    Ok(Json(serde_json::json!({
        "project": project,
        "status": "removed"
    })))
}

async fn logs_stack_h(
    State(docker): State<Docker>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();

    let mut logs: Vec<serde_json::Value> = Vec::new();

    for c in &containers {
        let labels = c.labels.as_ref();
        let proj = labels.and_then(|l| l.get(LABEL_COMPOSE_PROJECT)).cloned();
        if proj.as_deref() != Some(&project) {
            continue;
        }
        let service = labels
            .and_then(|l| l.get(LABEL_COMPOSE_SERVICE))
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        let name = c
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| strip_name(n))
            .unwrap_or_default();

        let opts = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: "50".into(),
            timestamps: true,
            ..Default::default()
        };

        let log_lines: Vec<String> = docker
            .logs(&name, Some(opts))
            .take(50)
            .filter_map(|r| async move {
                match r {
                    Ok(log) => {
                        let text = log.to_string();
                        // Remove trailing newline from each line
                        Some(text.trim_end().to_string())
                    }
                    Err(_) => None,
                }
            })
            .collect()
            .await;

        logs.push(serde_json::json!({
            "service": service,
            "container": name,
            "lines": log_lines,
        }));
    }

    Json(serde_json::json!({
        "project": project,
        "services": logs,
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/stacks", get(list_stacks_h))
        .route("/api/stacks/{project}/update", post(update_stack_h))
        .route("/api/stacks/{project}/down", post(down_stack_h))
        .route("/api/stacks/{project}/logs", get(logs_stack_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_podman_available() -> bool {
        std::env::var("DOCKER_HOST").is_ok()
            || std::path::Path::new("/run/user/1000/podman/podman.sock").exists()
    }

    async fn podman_client() -> Docker {
        let socket = std::env::var("DOCKER_HOST")
            .unwrap_or_else(|_| "unix:///run/user/1000/podman/podman.sock".to_string());
        Docker::connect_with_local(&socket, 120, bollard::API_DEFAULT_VERSION)
            .expect("Failed to connect to Podman socket")
    }

    // ── Pure helpers (no Docker) ─────────────────────────────

    fn container(
        id: &str,
        name: &str,
        project: &str,
        service: &str,
    ) -> bollard::models::ContainerSummary {
        let mut labels = std::collections::HashMap::new();
        labels.insert(LABEL_COMPOSE_PROJECT.to_string(), project.to_string());
        labels.insert(LABEL_COMPOSE_SERVICE.to_string(), service.to_string());
        bollard::models::ContainerSummary {
            id: Some(id.to_string()),
            names: Some(vec![format!("/{}", name)]),
            labels: Some(labels),
            ..Default::default()
        }
    }

    fn network(id: &str, project: &str) -> bollard::models::Network {
        let mut labels = std::collections::HashMap::new();
        labels.insert(LABEL_COMPOSE_PROJECT.to_string(), project.to_string());
        bollard::models::Network {
            id: Some(id.to_string()),
            labels: Some(labels),
            ..Default::default()
        }
    }

    #[test]
    fn test_select_project_containers_filters_by_label() {
        let containers = vec![
            container("id1", "app_web_1", "app", "web"),
            container("id2", "app_db_1", "app", "db"),
            container("id3", "other_web_1", "other", "web"),
        ];
        let selected = select_project_containers(&containers, "app");
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|c| c.id.as_deref() != Some("id3")));
    }

    #[test]
    fn test_project_services_unique_in_encounter_order() {
        let containers = vec![
            container("id1", "app_web_1", "app", "web"),
            container("id2", "app_db_1", "app", "db"),
            container("id3", "app_web_2", "app", "web"),
            container("id4", "other_db_1", "other", "db"),
        ];
        let services = project_services(&containers, "app");
        assert_eq!(services, vec!["web".to_string(), "db".to_string()]);
    }

    #[test]
    fn test_build_down_plan_produces_stop_remove_sequence() {
        let containers = vec![
            container("id1", "app_web_1", "app", "web"),
            container("id2", "app_db_1", "app", "db"),
            container("id3", "other_web_1", "other", "web"),
        ];
        let plan = build_down_plan(&containers, "app");
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan[0],
            DownTarget {
                id: "id1".into(),
                name: "app_web_1".into()
            }
        );
        assert_eq!(
            plan[1],
            DownTarget {
                id: "id2".into(),
                name: "app_db_1".into()
            }
        );
    }

    #[test]
    fn test_build_down_plan_empty_for_unknown_project() {
        let containers = vec![container("id1", "app_web_1", "app", "web")];
        assert!(build_down_plan(&containers, "nope").is_empty());
    }

    #[test]
    fn test_project_networks_filters_by_label() {
        let networks = vec![
            network("net1", "app"),
            network("net2", "other"),
            network("net3", "app"),
        ];
        let selected = project_networks(&networks, "app");
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|n| n.id.as_deref() != Some("net2")));
    }

    // ── No external process (grep-asert) ─────────────────────

    fn read_source(relative: &str) -> String {
        std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative))
            .expect("source file readable")
    }

    #[test]
    fn test_stacks_source_does_not_invoke_docker_cli() {
        let src = read_source("src/stacks.rs");
        let new_cmd = concat!("Command::new", "(\"docker\")");
        let proc_cmd = concat!("process::", "Command");
        assert!(
            !src.contains(new_cmd),
            "stacks.rs todavía invoca el CLI docker"
        );
        assert!(
            !src.contains(proc_cmd),
            "stacks.rs todavía usa el módulo de procesos para invocar docker"
        );
    }

    #[test]
    fn test_handlers_source_does_not_invoke_docker_cli() {
        let src = read_source("src/updates/handlers.rs");
        let new_cmd = concat!("Command::new", "(\"docker\")");
        assert!(
            !src.contains(new_cmd),
            "handlers.rs todavía invoca el CLI docker"
        );
    }

    #[test]
    fn test_scheduler_source_does_not_invoke_docker_cli() {
        let src = read_source("src/workers/scheduler.rs");
        let new_cmd = concat!("Command::new", "(\"docker\")");
        let proc_cmd = concat!("process::", "Command");
        assert!(
            !src.contains(new_cmd),
            "scheduler.rs todavía invoca el CLI docker"
        );
        assert!(
            !src.contains(proc_cmd),
            "scheduler.rs todavía usa el módulo de procesos"
        );
    }

    // ── stack history entry ──────────────────────────────────

    #[test]
    fn test_stack_history_entry_fields() {
        let entry =
            stack_history_entry("app_web_1", "nginx:latest", "sha256:old", "sha256:new", 42);
        assert_eq!(entry.container, "app_web_1");
        assert_eq!(entry.image, "nginx:latest");
        assert_eq!(entry.old_digest, "sha256:old");
        assert_eq!(entry.new_digest, "sha256:new");
        assert_eq!(entry.status, "stack-update");
        assert_eq!(entry.duration_ms, 42);
        assert!(!entry.timestamp.is_empty());
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
                assert!(["running", "exited", "paused", "created"].contains(&svc.state.as_str()));
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
