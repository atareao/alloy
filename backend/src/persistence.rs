use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc;

// ── Synchronous JSON loader ────────────────────────────────

pub fn load_json<T: serde::de::DeserializeOwned>(path: &str) -> Vec<T> {
    match fs::read_to_string(path) {
        Ok(content) => {
            // Try as Vec<T> first, fall back to single T wrapped in Vec
            match serde_json::from_str(&content) {
                Ok(data) => data,
                Err(_) => match serde_json::from_str::<T>(&content) {
                    Ok(single) => vec![single],
                    Err(e) => {
                        tracing::warn!("load_json: error parsing {}: {}", path, e);
                        Vec::new()
                    }
                },
            }
        }
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    tracing::debug!("load_json: {} not found, using defaults", path);
                }
                _ => {
                    tracing::warn!("load_json: error reading {}: {}", path, e);
                }
            }
            Vec::new()
        }
    }
}

// ── Buffered JSON Writer ───────────────────────────────────

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
        // Ensure parent directory exists (e.g. data/ on first write)
        if let Some(parent) = Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(path, data) {
            tracing::warn!("json_writer: error writing {}: {}", path, e);
        } else {
            tracing::debug!("json_writer: flushed {} ({})", path, unique.len());
        }
    }
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UpdateHistoryEntry;
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
}
