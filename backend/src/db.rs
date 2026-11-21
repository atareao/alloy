use deadpool_sqlite::{Config as PoolConfig, Runtime};
use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::HashMap;

use crate::models::*;

pub type DbPool = deadpool_sqlite::Pool;

/// Initialize database: create tables, set WAL mode, return async connection pool
pub async fn init_db(path: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    let pool = PoolConfig::new(path).create_pool(Runtime::Tokio1)?;
    let obj = pool.get().await?;
    let conn = obj.lock().unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    apply_schema(&conn)?;

    Ok(pool)
}

/// Esquema de la base de datos (tablas) + migraciones idempotentes.
///
/// Compartido por `init_db()` (producción) y `test_pool()` (tests) para que el esquema de
/// test no pueda divergir del real (p. ej. columnas añadidas por migraciones).
fn apply_schema(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS containers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            image TEXT NOT NULL,
            image_tag TEXT NOT NULL DEFAULT '',
            size_mb REAL NOT NULL DEFAULT 0.0,
            state TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT '',
            ports TEXT NOT NULL DEFAULT '[]',
            traefik_url TEXT,
            compose_project TEXT,
            has_update INTEGER NOT NULL DEFAULT 0,
            registry_url TEXT NOT NULL DEFAULT '',
            last_check TEXT,
            next_check TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS container_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            container TEXT NOT NULL,
            event_type TEXT NOT NULL,
            status TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS update_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            container TEXT NOT NULL,
            image TEXT NOT NULL,
            old_digest TEXT NOT NULL DEFAULT '',
            new_digest TEXT NOT NULL DEFAULT '',
            timestamp TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT '',
            duration_ms INTEGER NOT NULL DEFAULT 0,
            attempts INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS update_policies (
            container TEXT PRIMARY KEY,
            action TEXT NOT NULL DEFAULT 'none',
            cleanup_old_image INTEGER NOT NULL DEFAULT 0,
            rollback_on_failure INTEGER NOT NULL DEFAULT 0,
            notify_events INTEGER NOT NULL DEFAULT 0,
            max_start_attempts INTEGER,
            health_window_secs INTEGER
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS container_updating (
            container TEXT PRIMARY KEY
        );
        ",
    )?;

    // Migraciones idempotentes (ignorar el error si la columna ya existe).
    let _ = conn.execute_batch(
        "ALTER TABLE update_policies ADD COLUMN notify_events INTEGER NOT NULL DEFAULT 0;",
    );
    let _ = conn.execute_batch("ALTER TABLE containers ADD COLUMN last_check TEXT;");
    let _ = conn.execute_batch("ALTER TABLE containers ADD COLUMN next_check TEXT;");
    let _ = conn.execute_batch(
        "ALTER TABLE containers ADD COLUMN last_remote_digest TEXT NOT NULL DEFAULT '';",
    );
    let _ = conn.execute_batch(
        "ALTER TABLE containers ADD COLUMN last_pulled_digest TEXT NOT NULL DEFAULT '';",
    );
    // update-rollback-guard: intentos de arranque en el historial y overrides por política.
    let _ = conn.execute_batch(
        "ALTER TABLE update_history ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;",
    );
    let _ =
        conn.execute_batch("ALTER TABLE update_policies ADD COLUMN max_start_attempts INTEGER;");
    let _ =
        conn.execute_batch("ALTER TABLE update_policies ADD COLUMN health_window_secs INTEGER;");
    // persist-terminal-update-state: acelera la consulta del último estado por
    // contenedor (`load_latest_history_per_container`) usada al restaurar los
    // guards en el arranque.
    let _ = conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_update_history_container_ts
         ON update_history(container, timestamp DESC, id DESC);",
    );
    Ok(())
}

/// Crea una pool de test `:memory:` a partir de una `PoolConfig` y le aplica el
/// esquema real (`apply_schema`), de modo que el esquema de test no pueda divergir
/// del de producción. La conexión en memoria vive mientras la pool la retenga.
#[cfg(test)]
fn pool_with_schema(builder: PoolConfig) -> DbPool {
    let pool = builder
        .create_pool(Runtime::Tokio1)
        .expect("Failed to create test pool");
    // `apply_schema` es síncrono; se ejecuta en su propio hilo con runtime para no
    // exigir un contexto Tokio al llamante.
    let conn = std::thread::spawn({
        let pool = pool.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let obj = pool.get().await.unwrap();
                let conn = obj.lock().unwrap();
                apply_schema(&conn).expect("Failed to create test schema");
            });
        }
    });
    conn.join().unwrap();
    pool
}

#[cfg(test)]
pub fn test_pool() -> DbPool {
    pool_with_schema(PoolConfig::new(":memory:"))
}

/// Pool de test con `max_size` conexiones. Con `max_size == 1` permite saturar la
/// pool (una única conexión ocupada por el test) para comprobar que ningún handler
/// bloquea el mutex de `Settings` mientras espera una conexión.
#[cfg(test)]
pub fn test_pool_with_max_size(max_size: usize) -> DbPool {
    // `deadpool_sqlite::PoolConfig` es el `deadpool::managed::PoolConfig` reexportado
    // (el `Config` de SQLite se importa como `PoolConfig` en la cabecera).
    let config = PoolConfig {
        path: ":memory:".into(),
        pool: Some(deadpool_sqlite::PoolConfig::new(max_size)),
    };
    pool_with_schema(config)
}

// ── Containers ───────────────────────────────────────────────

#[allow(dead_code)]
pub fn save_containers(conn: &Connection, containers: &[ContainerInfo]) -> SqlResult<()> {
    conn.execute("DELETE FROM containers", [])?;
    let mut stmt = conn.prepare(
        "INSERT INTO containers (id, name, image, image_tag, size_mb, state, status, ports, traefik_url, compose_project, has_update, registry_url, last_check, next_check, last_remote_digest, last_pulled_digest, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, datetime('now'))",
    )?;
    for c in containers {
        stmt.execute(params![
            c.id,
            c.name,
            c.image,
            c.image_tag,
            c.size_mb,
            c.state,
            c.status,
            serde_json::to_string(&c.ports).unwrap_or_default(),
            c.traefik_url,
            c.compose_project,
            c.has_update as i32,
            c.registry_url,
            c.last_check,
            c.next_check,
            c.last_remote_digest,
            c.last_pulled_digest,
        ])?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn load_containers(conn: &Connection) -> SqlResult<Vec<ContainerInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, image, image_tag, size_mb, state, status, ports, traefik_url, compose_project, has_update, registry_url, last_check, next_check, last_remote_digest, last_pulled_digest FROM containers ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        let ports_str: String = row.get(7)?;
        let ports: Vec<String> = serde_json::from_str(&ports_str).unwrap_or_default();
        let lrd: String = row.get(14)?;
        let lpd: String = row.get(15)?;
        Ok(ContainerInfo {
            id: row.get(0)?,
            name: row.get(1)?,
            image: row.get(2)?,
            image_tag: row.get(3)?,
            size_mb: row.get(4)?,
            state: row.get(5)?,
            status: row.get(6)?,
            ports,
            traefik_url: row.get(8)?,
            compose_project: row.get(9)?,
            has_update: row.get::<_, i32>(10)? != 0,
            updating: false,
            registry_url: row.get(11)?,
            last_check: row.get(12)?,
            next_check: row.get(13)?,
            last_remote_digest: lrd,
            last_pulled_digest: lpd,
        })
    })?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

pub fn update_container_has_update(
    conn: &Connection,
    name: &str,
    has_update: bool,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE containers SET has_update = ?1, updated_at = datetime('now') WHERE name = ?2",
        params![has_update as i32, name],
    )?;
    Ok(())
}

pub fn update_container_last_remote_digest(
    conn: &Connection,
    name: &str,
    digest: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE containers SET last_remote_digest = ?1, updated_at = datetime('now') WHERE name = ?2",
        params![digest, name],
    )?;
    Ok(())
}

pub fn get_container_last_remote_digest(conn: &Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT last_remote_digest FROM containers WHERE name = ?1",
        params![name],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|d| !d.is_empty())
}

pub fn load_last_remote_digest_map(conn: &Connection) -> HashMap<String, String> {
    let mut stmt = match conn
        .prepare("SELECT name, last_remote_digest FROM containers WHERE last_remote_digest != ''")
    {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let rows = match stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let digest: String = row.get(1)?;
        Ok((name, digest))
    }) {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

// ── last_pulled_digest (local image cache) ──────────────────

pub fn update_container_last_pulled_digest(
    conn: &Connection,
    name: &str,
    digest: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE containers SET last_pulled_digest = ?1, updated_at = datetime('now') WHERE name = ?2",
        params![digest, name],
    )?;
    Ok(())
}

pub fn get_container_last_pulled_digest(conn: &Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT last_pulled_digest FROM containers WHERE name = ?1",
        params![name],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|d| !d.is_empty())
}

pub fn load_last_pulled_digest_map(conn: &Connection) -> HashMap<String, String> {
    let mut stmt = match conn
        .prepare("SELECT name, last_pulled_digest FROM containers WHERE last_pulled_digest != ''")
    {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let rows = match stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let digest: String = row.get(1)?;
        Ok((name, digest))
    }) {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}

pub fn update_container_check_times(
    conn: &Connection,
    name: &str,
    last_check: &str,
    next_check: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE containers SET last_check = ?1, next_check = ?2, updated_at = datetime('now') WHERE name = ?3",
        params![last_check, next_check, name],
    )?;
    Ok(())
}

// ── Container Updating ─────────────────────────────────────

#[allow(dead_code)]
pub fn set_updating(conn: &Connection, name: &str) -> SqlResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO container_updating (container) VALUES (?1)",
        params![name],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn clear_updating(conn: &Connection, name: &str) -> SqlResult<()> {
    conn.execute(
        "DELETE FROM container_updating WHERE container = ?1",
        params![name],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn list_updating(conn: &Connection) -> SqlResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT container FROM container_updating")?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        Ok(name)
    })?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

// ── Update History ───────────────────────────────────────────

#[allow(dead_code)]
pub fn save_update_history(conn: &Connection, entries: &[UpdateHistoryEntry]) -> SqlResult<()> {
    conn.execute("DELETE FROM update_history", [])?;
    let mut stmt = conn.prepare(
        "INSERT INTO update_history (container, image, old_digest, new_digest, timestamp, status, duration_ms, attempts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    for e in entries {
        stmt.execute(params![
            e.container,
            e.image,
            e.old_digest,
            e.new_digest,
            e.timestamp,
            e.status,
            e.duration_ms as i64,
            e.attempts as i64,
        ])?;
    }
    Ok(())
}

pub fn load_update_history(conn: &Connection) -> SqlResult<Vec<UpdateHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT container, image, old_digest, new_digest, timestamp, status, duration_ms, attempts FROM update_history ORDER BY timestamp DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(UpdateHistoryEntry {
            container: row.get(0)?,
            image: row.get(1)?,
            old_digest: row.get(2)?,
            new_digest: row.get(3)?,
            timestamp: row.get(4)?,
            status: row.get(5)?,
            duration_ms: row.get::<_, i64>(6)? as u64,
            attempts: row.get::<_, i64>(7)? as u32,
        })
    })?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

/// Último registro de historial por contenedor.
///
/// A diferencia de [`load_update_history`] (truncado a 100 filas), devuelve
/// exactamente una fila por contenedor —la más reciente— para reconstruir los
/// guards de arranque sin depender del truncado. El desempate por `id DESC`
/// desambigua filas con el mismo `timestamp`.
pub fn load_latest_history_per_container(conn: &Connection) -> SqlResult<Vec<UpdateHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT container, image, old_digest, new_digest, timestamp, status, duration_ms, attempts
         FROM update_history h
         WHERE h.id = (
             SELECT h2.id
             FROM update_history h2
             WHERE h2.container = h.container
             ORDER BY h2.timestamp DESC, h2.id DESC
             LIMIT 1
         )",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(UpdateHistoryEntry {
            container: row.get(0)?,
            image: row.get(1)?,
            old_digest: row.get(2)?,
            new_digest: row.get(3)?,
            timestamp: row.get(4)?,
            status: row.get(5)?,
            duration_ms: row.get::<_, i64>(6)? as u64,
            attempts: row.get::<_, i64>(7)? as u32,
        })
    })?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

pub fn append_update_history(conn: &Connection, entry: &UpdateHistoryEntry) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO update_history (container, image, old_digest, new_digest, timestamp, status, duration_ms, attempts)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            entry.container,
            entry.image,
            entry.old_digest,
            entry.new_digest,
            entry.timestamp,
            entry.status,
            entry.duration_ms as i64,
            entry.attempts as i64,
        ],
    )?;
    Ok(())
}

pub fn clear_update_history(conn: &Connection) -> SqlResult<()> {
    conn.execute("DELETE FROM update_history", [])?;
    Ok(())
}

// ── Update Policies ─────────────────────────────────────────

pub fn load_update_policies(conn: &Connection) -> SqlResult<Vec<UpdatePolicy>> {
    let mut stmt = conn.prepare(
        "SELECT container, action, cleanup_old_image, rollback_on_failure, notify_events, max_start_attempts, health_window_secs FROM update_policies",
    )?;
    let rows = stmt.query_map([], |row| {
        let action_str: String = row.get(1)?;
        Ok(UpdatePolicy {
            container: row.get(0)?,
            action: action_str.parse().unwrap_or(UpdateAction::None),
            cleanup_old_image: row.get::<_, i32>(2)? != 0,
            rollback_on_failure: row.get::<_, i32>(3)? != 0,
            notify_events: row.get::<_, i32>(4)? != 0,
            max_start_attempts: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
            health_window_secs: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
        })
    })?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

pub fn save_update_policy(conn: &Connection, policy: &UpdatePolicy) -> SqlResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO update_policies (container, action, cleanup_old_image, rollback_on_failure, notify_events, max_start_attempts, health_window_secs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            policy.container,
            policy.action.to_string(),
            policy.cleanup_old_image as i32,
            policy.rollback_on_failure as i32,
            policy.notify_events as i32,
            policy.max_start_attempts.map(|v| v as i64),
            policy.health_window_secs.map(|v| v as i64),
        ],
    )?;
    Ok(())
}

pub fn delete_update_policy(conn: &Connection, container: &str) -> SqlResult<()> {
    conn.execute(
        "DELETE FROM update_policies WHERE container = ?1",
        params![container],
    )?;
    Ok(())
}

// ── Settings ─────────────────────────────────────────────────

pub fn load_settings(conn: &Connection) -> SqlResult<Settings> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        Ok((key, value))
    })?;

    let mut map = std::collections::HashMap::new();
    for r in rows {
        let (k, v) = r?;
        map.insert(k, v);
    }

    Ok(Settings {
        telegram_token: map.get("telegram_token").cloned().filter(|s| !s.is_empty()),
        telegram_chat_id: map
            .get("telegram_chat_id")
            .cloned()
            .filter(|s| !s.is_empty()),
        telegram_topic_id: map
            .get("telegram_topic_id")
            .cloned()
            .filter(|s| !s.is_empty()),
        matrix_homeserver: map
            .get("matrix_homeserver")
            .cloned()
            .filter(|s| !s.is_empty()),
        matrix_token: map.get("matrix_token").cloned().filter(|s| !s.is_empty()),
        matrix_room: map.get("matrix_room").cloned().filter(|s| !s.is_empty()),
        webhook_url: map.get("webhook_url").cloned().filter(|s| !s.is_empty()),
        valet_url: map.get("valet_url").cloned().filter(|s| !s.is_empty()),
        valet_token: map.get("valet_token").cloned().filter(|s| !s.is_empty()),
        update_check_cron: map
            .get("update_check_cron")
            .cloned()
            .filter(|s| !s.is_empty()),
        update_check_enabled: map.get("update_check_enabled").and_then(|v| v.parse().ok()),
        update_check_notify: map.get("update_check_notify").and_then(|v| v.parse().ok()),
        default_update_action: map
            .get("default_update_action")
            .cloned()
            .filter(|s| !s.is_empty()),
        default_cleanup_old_image: map
            .get("default_cleanup_old_image")
            .and_then(|v| v.parse().ok()),
        default_rollback_on_failure: map
            .get("default_rollback_on_failure")
            .and_then(|v| v.parse().ok()),
        update_check_last_run_at: map
            .get("update_check_last_run_at")
            .cloned()
            .filter(|s| !s.is_empty()),
        pull_timeout_secs: map.get("pull_timeout_secs").and_then(|v| v.parse().ok()),
        check_interval_ms: map.get("check_interval_ms").and_then(|v| v.parse().ok()),
        update_max_start_attempts: map
            .get("update_max_start_attempts")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_UPDATE_MAX_START_ATTEMPTS),
        update_start_attempt_interval_secs: map
            .get("update_start_attempt_interval_secs")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_UPDATE_START_ATTEMPT_INTERVAL_SECS),
        update_health_window_secs: map
            .get("update_health_window_secs")
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_UPDATE_HEALTH_WINDOW_SECS),
        default_max_start_attempts: map
            .get("default_max_start_attempts")
            .and_then(|v| v.parse().ok()),
        default_health_window_secs: map
            .get("default_health_window_secs")
            .and_then(|v| v.parse().ok()),
    })
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> SqlResult<()> {
    let pairs = vec![
        ("telegram_token", settings.telegram_token.clone()),
        ("telegram_chat_id", settings.telegram_chat_id.clone()),
        ("telegram_topic_id", settings.telegram_topic_id.clone()),
        ("matrix_homeserver", settings.matrix_homeserver.clone()),
        ("matrix_token", settings.matrix_token.clone()),
        ("matrix_room", settings.matrix_room.clone()),
        ("webhook_url", settings.webhook_url.clone()),
        ("valet_url", settings.valet_url.clone()),
        ("valet_token", settings.valet_token.clone()),
        ("update_check_cron", settings.update_check_cron.clone()),
        (
            "update_check_enabled",
            settings.update_check_enabled.map(|v| v.to_string()),
        ),
        (
            "update_check_notify",
            settings.update_check_notify.map(|v| v.to_string()),
        ),
        (
            "default_update_action",
            settings.default_update_action.clone(),
        ),
        (
            "default_cleanup_old_image",
            settings.default_cleanup_old_image.map(|v| v.to_string()),
        ),
        (
            "default_rollback_on_failure",
            settings.default_rollback_on_failure.map(|v| v.to_string()),
        ),
        (
            "update_check_last_run_at",
            settings.update_check_last_run_at.clone(),
        ),
        (
            "pull_timeout_secs",
            settings.pull_timeout_secs.map(|v| v.to_string()),
        ),
        (
            "check_interval_ms",
            settings.check_interval_ms.map(|v| v.to_string()),
        ),
        (
            "update_max_start_attempts",
            Some(settings.update_max_start_attempts.to_string()),
        ),
        (
            "update_start_attempt_interval_secs",
            Some(settings.update_start_attempt_interval_secs.to_string()),
        ),
        (
            "update_health_window_secs",
            Some(settings.update_health_window_secs.to_string()),
        ),
        (
            "default_max_start_attempts",
            settings.default_max_start_attempts.map(|v| v.to_string()),
        ),
        (
            "default_health_window_secs",
            settings.default_health_window_secs.map(|v| v.to_string()),
        ),
    ];

    for (key, value) in pairs {
        match value {
            Some(v) => conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, v],
            )?,
            None => conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?,
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS containers (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, image TEXT NOT NULL DEFAULT '',
                image_tag TEXT NOT NULL DEFAULT '', size_mb REAL NOT NULL DEFAULT 0.0,
                state TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT '',
                ports TEXT NOT NULL DEFAULT '[]', traefik_url TEXT, compose_project TEXT,
                has_update INTEGER NOT NULL DEFAULT 0, registry_url TEXT NOT NULL DEFAULT '',
                last_check TEXT, next_check TEXT, last_remote_digest TEXT NOT NULL DEFAULT '',
                last_pulled_digest TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS update_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT, container TEXT NOT NULL,
                image TEXT NOT NULL, old_digest TEXT NOT NULL DEFAULT '',
                new_digest TEXT NOT NULL DEFAULT '', timestamp TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT '', duration_ms INTEGER NOT NULL DEFAULT 0,
                attempts INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS update_policies (
                container TEXT PRIMARY KEY, action TEXT NOT NULL DEFAULT 'none',
                cleanup_old_image INTEGER NOT NULL DEFAULT 0,
                rollback_on_failure INTEGER NOT NULL DEFAULT 0,
                notify_events INTEGER NOT NULL DEFAULT 0,
                max_start_attempts INTEGER, health_window_secs INTEGER
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY, value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS container_updating (
                container TEXT PRIMARY KEY
            );",
        )
        .expect("Failed to create test schema");
        conn
    }

    #[test]
    fn test_save_and_load_containers() {
        let conn = test_conn();
        let containers = vec![ContainerInfo {
            id: "abc123".into(),
            name: "nginx".into(),
            image: "nginx:latest".into(),
            image_tag: "latest".into(),
            size_mb: 10.5,
            state: "running".into(),
            status: "Up 2 hours".into(),
            ports: vec!["0.0.0.0:80:80".into()],
            traefik_url: Some("http://nginx.local".into()),
            compose_project: None,
            has_update: false,
            registry_url: "https://hub.docker.com/_/nginx".into(),
            updating: false,
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }];
        save_containers(&conn, &containers).unwrap();
        let loaded = load_containers(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "nginx");
    }

    #[test]
    fn test_save_containers_overwrites() {
        let conn = test_conn();
        let c1 = vec![ContainerInfo {
            id: "id1".into(),
            name: "web".into(),
            image: "nginx:1.0".into(),
            image_tag: "1.0".into(),
            size_mb: 5.0,
            state: "running".into(),
            status: "Up".into(),
            ports: vec![],
            traefik_url: None,
            compose_project: None,
            has_update: false,
            registry_url: String::new(),
            updating: false,
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }];
        save_containers(&conn, &c1).unwrap();
        let c2 = vec![ContainerInfo {
            id: "id2".into(),
            name: "web".into(),
            image: "nginx:2.0".into(),
            image_tag: "2.0".into(),
            size_mb: 6.0,
            state: "running".into(),
            status: "Up".into(),
            ports: vec![],
            traefik_url: None,
            compose_project: None,
            has_update: false,
            registry_url: String::new(),
            updating: false,
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }];
        save_containers(&conn, &c2).unwrap();
        let loaded = load_containers(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].image, "nginx:2.0");
    }

    #[test]
    fn test_load_containers_empty() {
        let conn = test_conn();
        let loaded = load_containers(&conn).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_update_container_has_update() {
        let conn = test_conn();
        let containers = vec![ContainerInfo {
            id: "abc".into(),
            name: "test".into(),
            image: "test:latest".into(),
            image_tag: "latest".into(),
            size_mb: 0.0,
            state: "".into(),
            status: "".into(),
            ports: vec![],
            traefik_url: None,
            compose_project: None,
            has_update: false,
            registry_url: String::new(),
            updating: false,
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }];
        save_containers(&conn, &containers).unwrap();
        update_container_has_update(&conn, "test", true).unwrap();
        let loaded = load_containers(&conn).unwrap();
        assert!(loaded[0].has_update);
    }

    #[test]
    fn test_set_and_clear_updating() {
        let conn = test_conn();
        set_updating(&conn, "nginx").unwrap();
        let list = list_updating(&conn).unwrap();
        assert!(list.contains(&"nginx".to_string()));
        clear_updating(&conn, "nginx").unwrap();
        let list = list_updating(&conn).unwrap();
        assert!(!list.contains(&"nginx".to_string()));
    }

    #[test]
    fn test_list_updating_multiple() {
        let conn = test_conn();
        set_updating(&conn, "a").unwrap();
        set_updating(&conn, "b").unwrap();
        let list = list_updating(&conn).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_save_and_load_update_history() {
        let conn = test_conn();
        let entries = vec![UpdateHistoryEntry {
            container: "nginx".into(),
            image: "nginx:latest".into(),
            old_digest: "abc".into(),
            new_digest: "def".into(),
            timestamp: "2024-01-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 1000,
            attempts: 0,
        }];
        save_update_history(&conn, &entries).unwrap();
        let loaded = load_update_history(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].container, "nginx");
    }

    #[test]
    fn test_append_update_history() {
        let conn = test_conn();
        let entry = UpdateHistoryEntry {
            container: "redis".into(),
            image: "redis:7".into(),
            old_digest: "old".into(),
            new_digest: "new".into(),
            timestamp: "2024-06-01T00:00:00".into(),
            status: "success".into(),
            duration_ms: 500,
            attempts: 0,
        };
        append_update_history(&conn, &entry).unwrap();
        let loaded = load_update_history(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].container, "redis");
    }

    #[test]
    fn test_clear_update_history() {
        let conn = test_conn();
        let entry = UpdateHistoryEntry {
            container: "c".into(),
            image: "img".into(),
            old_digest: "".into(),
            new_digest: "".into(),
            timestamp: "now".into(),
            status: "ok".into(),
            duration_ms: 0,
            attempts: 0,
        };
        append_update_history(&conn, &entry).unwrap();
        clear_update_history(&conn).unwrap();
        let loaded = load_update_history(&conn).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_policies_crud() {
        let conn = test_conn();
        let policy = UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: true,
            rollback_on_failure: false,
            notify_events: true,
            max_start_attempts: None,
            health_window_secs: None,
        };
        save_update_policy(&conn, &policy).unwrap();
        let loaded = load_update_policies(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].action, UpdateAction::PullRestart);
        assert!(loaded[0].notify_events);
    }

    #[test]
    fn test_delete_update_policy() {
        let conn = test_conn();
        let policy = UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: false,
            rollback_on_failure: false,
            notify_events: false,
            max_start_attempts: None,
            health_window_secs: None,
        };
        save_update_policy(&conn, &policy).unwrap();
        delete_update_policy(&conn, "nginx").unwrap();
        let loaded = load_update_policies(&conn).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_settings_save_load() {
        let conn = test_conn();
        let settings = Settings {
            telegram_token: Some("bot123".into()),
            telegram_chat_id: Some("chat456".into()),
            ..Default::default()
        };
        save_settings(&conn, &settings).unwrap();
        let loaded = load_settings(&conn).unwrap();
        assert_eq!(loaded.telegram_token.as_deref(), Some("bot123"));
    }

    #[test]
    fn test_load_settings_defaults() {
        let conn = test_conn();
        let settings = load_settings(&conn).unwrap();
        assert_eq!(settings.telegram_token, None);
    }

    // ── update-rollback-guard: settings / políticas / historial ───

    /// Sin valores persistidos, la guarda cae a los defaults globales (3/10/30).
    #[test]
    fn test_load_settings_guard_defaults() {
        let conn = test_conn();
        let settings = load_settings(&conn).unwrap();
        assert_eq!(settings.update_max_start_attempts, 3);
        assert_eq!(settings.update_start_attempt_interval_secs, 10);
        assert_eq!(settings.update_health_window_secs, 30);
    }

    /// Los 3 campos de la guarda hacen round-trip en settings.
    #[test]
    fn test_settings_guard_fields_roundtrip() {
        let conn = test_conn();
        let settings = Settings {
            update_max_start_attempts: 4,
            update_start_attempt_interval_secs: 15,
            update_health_window_secs: 45,
            ..Default::default()
        };
        save_settings(&conn, &settings).unwrap();

        let loaded = load_settings(&conn).unwrap();
        assert_eq!(loaded.update_max_start_attempts, 4);
        assert_eq!(loaded.update_start_attempt_interval_secs, 15);
        assert_eq!(loaded.update_health_window_secs, 45);
    }

    /// Los overrides por política (Some) y su ausencia (None) se persisten tal cual.
    #[test]
    fn test_update_policy_guard_overrides_roundtrip() {
        let conn = test_conn();
        let with_override = UpdatePolicy {
            container: "nginx".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: false,
            rollback_on_failure: true,
            notify_events: true,
            max_start_attempts: Some(5),
            health_window_secs: Some(60),
        };
        let without_override = UpdatePolicy {
            container: "redis".into(),
            max_start_attempts: None,
            health_window_secs: None,
            ..with_override.clone()
        };
        save_update_policy(&conn, &with_override).unwrap();
        save_update_policy(&conn, &without_override).unwrap();

        let loaded = load_update_policies(&conn).unwrap();
        let nginx = loaded.iter().find(|p| p.container == "nginx").unwrap();
        assert_eq!(nginx.max_start_attempts, Some(5));
        assert_eq!(nginx.health_window_secs, Some(60));
        let redis = loaded.iter().find(|p| p.container == "redis").unwrap();
        assert_eq!(redis.max_start_attempts, None);
        assert_eq!(redis.health_window_secs, None);
    }

    /// El nº de intentos y los digests del rollback persisten en el historial.
    #[test]
    fn test_update_history_attempts_roundtrip() {
        let conn = test_conn();
        let entry = UpdateHistoryEntry {
            container: "nginx".into(),
            image: "nginx:latest".into(),
            old_digest: "sha256:aaa".into(),
            new_digest: "sha256:bbb".into(),
            timestamp: "2026-01-01T00:00:00".into(),
            status: "rolled-back".into(),
            duration_ms: 4200,
            attempts: 3,
        };
        append_update_history(&conn, &entry).unwrap();

        let loaded = load_update_history(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].status, "rolled-back");
        assert_eq!(loaded[0].old_digest, "sha256:aaa");
        assert_eq!(loaded[0].new_digest, "sha256:bbb");
        assert_eq!(loaded[0].attempts, 3);
    }

    // ── Replay del último estado por contenedor (change:
    // persist-terminal-update-state) ─────────────────────────────────

    fn hist(
        container: &str,
        status: &str,
        new_digest: &str,
        timestamp: &str,
    ) -> UpdateHistoryEntry {
        UpdateHistoryEntry {
            container: container.into(),
            image: "img:latest".into(),
            old_digest: "sha256:old".into(),
            new_digest: new_digest.into(),
            timestamp: timestamp.into(),
            status: status.into(),
            duration_ms: 0,
            attempts: 0,
        }
    }

    /// `load_latest_history_per_container` devuelve exactamente una fila por
    /// contenedor —la más reciente— con semántica "último gana".
    #[test]
    fn test_load_latest_history_per_container_last_wins() {
        let conn = test_conn();
        append_update_history(
            &conn,
            &hist("app", "failed-stopped", "", "2026-01-01T00:00:00+00:00"),
        )
        .unwrap();
        append_update_history(
            &conn,
            &hist("app", "updated", "", "2026-01-02T00:00:00+00:00"),
        )
        .unwrap();
        append_update_history(
            &conn,
            &hist(
                "db",
                "rolled-back",
                "sha256:abc",
                "2026-01-01T12:00:00+00:00",
            ),
        )
        .unwrap();

        let latest = load_latest_history_per_container(&conn).unwrap();
        assert_eq!(latest.len(), 2, "una fila por contenedor");

        let app = latest.iter().find(|e| e.container == "app").unwrap();
        assert_eq!(app.status, "updated", "la fila posterior anula la terminal");

        let db = latest.iter().find(|e| e.container == "db").unwrap();
        assert_eq!(db.status, "rolled-back");
        assert_eq!(db.new_digest, "sha256:abc");
    }

    /// El replay resiste el truncado `LIMIT 100`: un estado terminal fuera de las
    /// 100 filas más recientes sigue recuperándose.
    #[test]
    fn test_load_latest_history_per_container_resists_limit_100() {
        let conn = test_conn();
        append_update_history(
            &conn,
            &hist("buried", "failed-stopped", "", "2020-01-01T00:00:00+00:00"),
        )
        .unwrap();
        for i in 0..120 {
            append_update_history(
                &conn,
                &hist(&format!("c{i}"), "updated", "", "2026-01-01T00:00:00+00:00"),
            )
            .unwrap();
        }

        // La lista truncada a 100 NO contiene al contenedor enterrado…
        let truncated = load_update_history(&conn).unwrap();
        assert_eq!(truncated.len(), 100);
        assert!(!truncated.iter().any(|e| e.container == "buried"));

        // …pero la consulta por-contenedor SÍ lo recupera.
        let latest = load_latest_history_per_container(&conn).unwrap();
        let buried = latest
            .iter()
            .find(|e| e.container == "buried")
            .expect("la fila terminal debe recuperarse");
        assert_eq!(buried.status, "failed-stopped");
    }

    /// Flujo completo: la consulta por-contenedor alimenta el replay y
    /// reconstruye el guard aunque la fila terminal quede fuera del LIMIT 100.
    #[test]
    fn test_restore_from_replayed_history_resists_truncation() {
        use crate::updates::health_guard::{restore_guards_from_history, STATUS_FAILED_STOPPED};

        let conn = test_conn();
        append_update_history(
            &conn,
            &hist(
                "buried",
                STATUS_FAILED_STOPPED,
                "",
                "2020-01-01T00:00:00+00:00",
            ),
        )
        .unwrap();
        for i in 0..120 {
            append_update_history(
                &conn,
                &hist(&format!("c{i}"), "updated", "", "2026-01-01T00:00:00+00:00"),
            )
            .unwrap();
        }

        let latest = load_latest_history_per_container(&conn).unwrap();
        let present: std::collections::HashSet<String> =
            std::collections::HashSet::from(["buried".to_string()]);
        let (terminal, _) = restore_guards_from_history(&latest, &present);
        assert!(terminal.contains("buried"));
    }

    /// A igual `timestamp`, el desempate por `id DESC` hace ganar la fila
    /// insertada más tarde (los timestamps tienen granularidad de segundo).
    #[test]
    fn test_load_latest_history_per_container_tie_break_by_id() {
        let conn = test_conn();
        let ts = "2026-01-01T00:00:00+00:00";
        append_update_history(&conn, &hist("app", "failed-stopped", "", ts)).unwrap();
        append_update_history(&conn, &hist("app", "updated", "", ts)).unwrap();

        let latest = load_latest_history_per_container(&conn).unwrap();
        assert_eq!(latest.len(), 1);
        assert_eq!(
            latest[0].status, "updated",
            "a igual timestamp, gana el id mayor (fila insertada más tarde)"
        );
    }

    // ── telegram_topic_id round-trip (change: telegram-topic-id) ───

    /// `save_settings`/`load_settings` must round-trip the Telegram topic like
    /// any other string key.
    #[test]
    fn test_settings_topic_id_save_load_roundtrip() {
        let conn = test_conn();
        let settings = Settings {
            telegram_token: Some("bot123".into()),
            telegram_chat_id: Some("chat456".into()),
            telegram_topic_id: Some("42".into()),
            ..Default::default()
        };
        save_settings(&conn, &settings).unwrap();

        let loaded = load_settings(&conn).unwrap();
        assert_eq!(loaded.telegram_topic_id.as_deref(), Some("42"));
    }

    /// Persisting `None` must delete the stored key, so a later load yields
    /// `None` again (same `None ⇒ DELETE` semantics as the rest).
    #[test]
    fn test_settings_topic_id_none_deletes_row() {
        let conn = test_conn();
        let with_topic = Settings {
            telegram_topic_id: Some("42".into()),
            ..Default::default()
        };
        save_settings(&conn, &with_topic).unwrap();

        let without_topic = Settings::default();
        save_settings(&conn, &without_topic).unwrap();

        let loaded = load_settings(&conn).unwrap();
        assert_eq!(loaded.telegram_topic_id, None);
    }

    #[test]
    fn test_get_container_last_remote_digest_nonexistent() {
        let conn = test_conn();
        let result = get_container_last_remote_digest(&conn, "nonexistent");
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_container_last_remote_digest_exists() {
        let conn = test_conn();
        // Insert a container first
        let containers = vec![ContainerInfo {
            id: "test123".into(),
            name: "test-container".into(),
            image: "test:latest".into(),
            image_tag: "latest".into(),
            size_mb: 0.0,
            state: "running".into(),
            status: "Up".into(),
            ports: vec![],
            traefik_url: None,
            compose_project: None,
            has_update: false,
            registry_url: String::new(),
            updating: false,
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }];
        save_containers(&conn, &containers).unwrap();

        // Initially empty digest → None
        assert_eq!(
            get_container_last_remote_digest(&conn, "test-container"),
            None,
        );

        // Set a digest → Some
        update_container_last_remote_digest(&conn, "test-container", "sha256:abc123").unwrap();
        assert_eq!(
            get_container_last_remote_digest(&conn, "test-container"),
            Some("sha256:abc123".into()),
        );
    }

    // ── last_pulled_digest (pull-cache-digest) ──────────────

    fn make_container(name: &str) -> ContainerInfo {
        ContainerInfo {
            id: name.into(),
            name: name.into(),
            image: format!("{}:latest", name),
            image_tag: "latest".into(),
            size_mb: 0.0,
            state: "running".into(),
            status: "Up".into(),
            ports: vec![],
            traefik_url: None,
            compose_project: None,
            has_update: false,
            registry_url: String::new(),
            updating: false,
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
            last_pulled_digest: String::new(),
        }
    }

    /// Scenario: Migración sobre BD existente — init_db adds the column with
    /// default '' and is idempotent (safe to run twice).
    #[tokio::test]
    async fn test_init_db_adds_last_pulled_digest_column() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("alloy-migration.db");
        let path_str = path.to_str().expect("path str");

        let pool = init_db(path_str).await.expect("first init_db");
        // Second call must not fail (idempotent migration).
        let pool2 = init_db(path_str).await.expect("second init_db");

        let conn = pool2.get().await.expect("pool conn");
        let guard = conn.lock().expect("lock");
        let mut stmt = guard
            .prepare("PRAGMA table_info(containers)")
            .expect("pragma");
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query_map")
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            columns.contains(&"last_pulled_digest".to_string()),
            "last_pulled_digest column missing; columns={:?}",
            columns
        );
        drop(pool);
    }

    /// Scenario: Pull exitoso persiste el digest — update/get roundtrip.
    #[test]
    fn test_update_and_get_last_pulled_digest() {
        let conn = test_conn();
        save_containers(&conn, &[make_container("test-container")]).unwrap();

        // Initially empty digest → None
        assert_eq!(
            get_container_last_pulled_digest(&conn, "test-container"),
            None
        );

        // Set a digest → Some
        update_container_last_pulled_digest(&conn, "test-container", "sha256:bbb").unwrap();
        assert_eq!(
            get_container_last_pulled_digest(&conn, "test-container"),
            Some("sha256:bbb".into()),
        );
    }

    #[test]
    fn test_get_container_last_pulled_digest_nonexistent() {
        let conn = test_conn();
        assert_eq!(get_container_last_pulled_digest(&conn, "nonexistent"), None);
    }

    /// Scenario: load_last_pulled_digest_map omite valores vacíos.
    #[test]
    fn test_load_last_pulled_digest_map_omits_empty() {
        let conn = test_conn();
        save_containers(&conn, &[make_container("cached"), make_container("empty")]).unwrap();
        update_container_last_pulled_digest(&conn, "cached", "sha256:bbb").unwrap();

        let map = load_last_pulled_digest_map(&conn);
        assert_eq!(map.len(), 1, "only non-empty digests must be included");
        assert_eq!(map.get("cached"), Some(&"sha256:bbb".to_string()));
        assert_eq!(map.get("empty"), None);
    }
}
