use rusqlite::{params, Connection, Result as SqlResult};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::*;

pub type DbPool = Arc<Mutex<Connection>>;

static GLOBAL_DB: std::sync::OnceLock<DbPool> = std::sync::OnceLock::new();

/// Initialize the global database pool. Must be called once at startup.
pub fn init_global(pool: DbPool) {
    GLOBAL_DB
        .set(pool)
        .unwrap_or_else(|_| panic!("db::init_global called more than once"));
}

/// Initialize a temporary in-memory database for testing.
/// Returns a pool that can be used for test assertions.
pub fn init_test_db() -> DbPool {
    let conn = Connection::open_in_memory().expect("Failed to create test database");
    // Run schema creation
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS containers (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, image TEXT NOT NULL DEFAULT '',
            image_tag TEXT NOT NULL DEFAULT '', size_mb REAL NOT NULL DEFAULT 0.0,
            state TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT '',
            ports TEXT NOT NULL DEFAULT '[]', traefik_url TEXT, compose_project TEXT,
            has_update INTEGER NOT NULL DEFAULT 0, registry_url TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS update_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT, container TEXT NOT NULL,
            image TEXT NOT NULL, old_digest TEXT NOT NULL DEFAULT '',
            new_digest TEXT NOT NULL DEFAULT '', timestamp TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT '', duration_ms INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS update_policies (
            container TEXT PRIMARY KEY, action TEXT NOT NULL DEFAULT 'none',
            cleanup_old_image INTEGER NOT NULL DEFAULT 0,
            rollback_on_failure INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY, value TEXT NOT NULL
        );",
    )
    .expect("Failed to create test schema");
    let pool: DbPool = Arc::new(Mutex::new(conn));
    pool
}

/// Get the global database pool.
/// If not initialized yet (e.g. in tests), initializes an in-memory database automatically.
pub fn global() -> &'static DbPool {
    GLOBAL_DB.get_or_init(|| {
        tracing::warn!("db::global() called before init_global — using in-memory fallback");
        init_test_db()
    })
}

/// Initialize database: create tables, set WAL mode
pub fn init_db(path: &str) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

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
            duration_ms INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS update_policies (
            container TEXT PRIMARY KEY,
            action TEXT NOT NULL DEFAULT 'none',
            cleanup_old_image INTEGER NOT NULL DEFAULT 0,
            rollback_on_failure INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )?;

    Ok(conn)
}

// ── Containers ───────────────────────────────────────────────

#[allow(dead_code)]
pub fn save_containers(conn: &Connection, containers: &[ContainerInfo]) -> SqlResult<()> {
    conn.execute("DELETE FROM containers", [])?;
    let mut stmt = conn.prepare(
        "INSERT INTO containers (id, name, image, image_tag, size_mb, state, status, ports, traefik_url, compose_project, has_update, registry_url, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))",
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
        ])?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn load_containers(conn: &Connection) -> SqlResult<Vec<ContainerInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, image, image_tag, size_mb, state, status, ports, traefik_url, compose_project, has_update, registry_url FROM containers ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        let ports_str: String = row.get(7)?;
        let ports: Vec<String> = serde_json::from_str(&ports_str).unwrap_or_default();
        let has_update_int: i32 = row.get(10)?;
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
            has_update: has_update_int != 0,
            monitored: false,
            registry_url: row.get(11)?,
        })
    })?;
    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

#[allow(dead_code)]
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

// ── Update History ───────────────────────────────────────────

#[allow(dead_code)]
pub fn save_update_history(conn: &Connection, entries: &[UpdateHistoryEntry]) -> SqlResult<()> {
    conn.execute("DELETE FROM update_history", [])?;
    let mut stmt = conn.prepare(
        "INSERT INTO update_history (container, image, old_digest, new_digest, timestamp, status, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for e in entries {
        stmt.execute(params![
            e.container,
            e.image,
            e.old_digest,
            e.new_digest,
            e.timestamp,
            e.status,
            e.duration_ms,
        ])?;
    }
    Ok(())
}

pub fn load_update_history(conn: &Connection) -> SqlResult<Vec<UpdateHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT container, image, old_digest, new_digest, timestamp, status, duration_ms FROM update_history ORDER BY timestamp DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(UpdateHistoryEntry {
            container: row.get(0)?,
            image: row.get(1)?,
            old_digest: row.get(2)?,
            new_digest: row.get(3)?,
            timestamp: row.get(4)?,
            status: row.get(5)?,
            duration_ms: row.get(6)?,
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
        "INSERT INTO update_history (container, image, old_digest, new_digest, timestamp, status, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            entry.container,
            entry.image,
            entry.old_digest,
            entry.new_digest,
            entry.timestamp,
            entry.status,
            entry.duration_ms,
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
        "SELECT container, action, cleanup_old_image, rollback_on_failure FROM update_policies",
    )?;
    let rows = stmt.query_map([], |row| {
        let action_str: String = row.get(1)?;
        Ok(UpdatePolicy {
            container: row.get(0)?,
            action: action_str.parse().unwrap_or(UpdateAction::None),
            cleanup_old_image: row.get::<_, i32>(2)? != 0,
            rollback_on_failure: row.get::<_, i32>(3)? != 0,
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
        "INSERT OR REPLACE INTO update_policies (container, action, cleanup_old_image, rollback_on_failure)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            policy.container,
            policy.action.to_string(),
            policy.cleanup_old_image as i32,
            policy.rollback_on_failure as i32,
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
        auto_update_enabled: map.get("auto_update_enabled").and_then(|v| v.parse().ok()),
        auto_update_interval_hours: map
            .get("auto_update_interval_hours")
            .and_then(|v| v.parse().ok()),
        telegram_token: map.get("telegram_token").cloned().filter(|s| !s.is_empty()),
        telegram_chat_id: map
            .get("telegram_chat_id")
            .cloned()
            .filter(|s| !s.is_empty()),
        matrix_homeserver: map
            .get("matrix_homeserver")
            .cloned()
            .filter(|s| !s.is_empty()),
        matrix_token: map.get("matrix_token").cloned().filter(|s| !s.is_empty()),
        matrix_room: map.get("matrix_room").cloned().filter(|s| !s.is_empty()),
        webhook_url: map.get("webhook_url").cloned().filter(|s| !s.is_empty()),
        monitored_containers: serde_json::from_str(
            &map.get("monitored_containers").cloned().unwrap_or_default(),
        )
        .unwrap_or_default(),
        update_check_cron: map
            .get("update_check_cron")
            .cloned()
            .filter(|s| !s.is_empty()),
        update_check_enabled: map.get("update_check_enabled").and_then(|v| v.parse().ok()),
        update_check_notify: map.get("update_check_notify").and_then(|v| v.parse().ok()),
    })
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> SqlResult<()> {
    let pairs = vec![
        (
            "auto_update_enabled",
            settings.auto_update_enabled.map(|v| v.to_string()),
        ),
        (
            "auto_update_interval_hours",
            settings.auto_update_interval_hours.map(|v| v.to_string()),
        ),
        ("telegram_token", settings.telegram_token.clone()),
        ("telegram_chat_id", settings.telegram_chat_id.clone()),
        ("matrix_homeserver", settings.matrix_homeserver.clone()),
        ("matrix_token", settings.matrix_token.clone()),
        ("matrix_room", settings.matrix_room.clone()),
        ("webhook_url", settings.webhook_url.clone()),
        (
            "monitored_containers",
            Some(serde_json::to_string(&settings.monitored_containers).unwrap_or_default()),
        ),
        ("update_check_cron", settings.update_check_cron.clone()),
        (
            "update_check_enabled",
            settings.update_check_enabled.map(|v| v.to_string()),
        ),
        (
            "update_check_notify",
            settings.update_check_notify.map(|v| v.to_string()),
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
