use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{
    exec::{CreateExecOptions, StartExecOptions},
    Docker,
};
use futures::{future, StreamExt};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;

use crate::containers::find_container_by_name;
use crate::models::{AppError, TerminalInput};
use crate::state::AppState;

type TerminalSessions = Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>;

fn is_dangerous_command(input: &str) -> bool {
    let dangerous = ["rm -rf /", "dd if=", "mkfs.", "> /dev/", ":(){ :|:& };:"];
    dangerous.iter().any(|d| input.contains(d))
}

#[allow(clippy::type_complexity)]
async fn sse_terminal_h(
    State(docker): State<Docker>,
    State(terminal_tx): State<TerminalSessions>,
    Path(name): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let _found = find_container_by_name(&docker, &name).await?;
    let rx = {
        let mut sessions = terminal_tx.lock().await;
        let tx = sessions.entry(name.clone()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(256);
            tx
        });
        tx.subscribe()
    };
    let cleanup_name = name.clone();
    let cleanup_sessions = terminal_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        let mut sessions = cleanup_sessions.lock().await;
        if let Some(tx) = sessions.get(&cleanup_name) {
            if tx.receiver_count() == 0 {
                sessions.remove(&cleanup_name);
                tracing::debug!("🧹 Cleaned terminal session '{}'", cleanup_name);
            }
        }
    });
    let stream = BroadcastStream::new(rx).filter_map(|r| match r {
        Ok(output) => future::ready(Some(Ok(Event::default().event("output").data(output)))),
        Err(_) => future::ready(None),
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn terminal_input_h(
    State(docker): State<Docker>,
    State(terminal_tx): State<TerminalSessions>,
    Path(name): Path<String>,
    Json(body): Json<TerminalInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let cid = container
        .id
        .as_deref()
        .ok_or_else(|| AppError::NotFound("no container id".into()))?;
    let input = body.input.trim();
    if is_dangerous_command(input) {
        tracing::warn!("⚠️ BLOCKED dangerous command on '{}': {}", name, input);
        return Err(AppError::Internal("Command blocked for safety".into()));
    }
    tracing::info!("🔧 Terminal exec on '{}': {}", name, input);
    let exec_opts = CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        attach_stdin: Some(false),
        cmd: Some(vec!["/bin/sh", "-c", &body.input]),
        tty: Some(false),
        ..Default::default()
    };
    let exec = docker
        .create_exec(cid, exec_opts)
        .await
        .map_err(|e| AppError::Docker(format!("create_exec: {}", e)))?;
    let exec_id = exec.id;
    let output_result = docker
        .start_exec(&exec_id, None::<StartExecOptions>)
        .await
        .map_err(|e| AppError::Docker(format!("start_exec: {}", e)))?;
    let mut output = String::new();
    if let bollard::exec::StartExecResults::Attached {
        output: mut stream, ..
    } = output_result
    {
        while let Some(item) = stream.next().await {
            match item {
                Ok(bollard::container::LogOutput::StdOut { message })
                | Ok(bollard::container::LogOutput::StdErr { message })
                | Ok(bollard::container::LogOutput::Console { message }) => {
                    output.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }
    }
    if let Some(tx) = terminal_tx.lock().await.get(&name) {
        let _ = tx.send(output.clone());
    }
    Ok(Json(serde_json::json!({"output": output})))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/terminal/{name}", get(sse_terminal_h))
        .route("/api/terminal/{name}/input", post(terminal_input_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dangerous_rm_rf() {
        assert!(is_dangerous_command("rm -rf /"));
        assert!(is_dangerous_command("rm -rf /var"));
        assert!(is_dangerous_command("sudo rm -rf /"));
    }

    #[test]
    fn test_is_dangerous_dd() {
        assert!(is_dangerous_command("dd if=/dev/zero of=/dev/sda"));
        assert!(is_dangerous_command("dd if=/dev/random"));
    }

    #[test]
    fn test_is_dangerous_mkfs() {
        assert!(is_dangerous_command("mkfs.ext4 /dev/sda1"));
        assert!(is_dangerous_command("/sbin/mkfs.btrfs /dev/sdb"));
    }

    #[test]
    fn test_is_dangerous_dev_null_write() {
        assert!(is_dangerous_command("echo > /dev/sda"));
        assert!(is_dangerous_command("cat > /dev/null"));
    }

    #[test]
    fn test_is_dangerous_fork_bomb() {
        assert!(is_dangerous_command(":(){ :|:& };:"));
    }

    #[test]
    fn test_is_dangerous_safe_commands() {
        assert!(!is_dangerous_command("ls -la"));
        assert!(!is_dangerous_command("echo hello"));
        assert!(!is_dangerous_command("docker ps"));
        assert!(!is_dangerous_command("cat /etc/passwd"));
        assert!(!is_dangerous_command("top"));
        assert!(!is_dangerous_command("ps aux"));
    }

    #[test]
    fn test_is_dangerous_empty() {
        assert!(!is_dangerous_command(""));
    }

    #[test]
    fn test_is_dangerous_substring_safe() {
        assert!(!is_dangerous_command("rm"));
        assert!(!is_dangerous_command("dd"));
        assert!(!is_dangerous_command("mkfs"));
    }
}
