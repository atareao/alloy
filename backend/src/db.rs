use deadpool_sqlite::{Config as PoolConfig, Runtime};
use rusqlite::{params, Connection, Result as SqlResult};

use crate::models::*;

pub type DbPool = deadpool_sqlite::Pool;

/// Initialize database: create tables, set WAL mode, return async connection pool
pub async fn init_db(path: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    let pool = PoolConfig::new(path).create_pool(Runtime::Tokio1)?;
    let obj = pool.get().await?;
    let conn = obj.lock().unwrap();
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
            rollback_on_failure INTEGER NOT NULL DEFAULT 0,
            notify_events INTEGER NOT NULL DEFAULT 0
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

    let _ = conn.execute_batch(
        "ALTER TABLE update_policies ADD COLUMN notify_events INTEGER NOT NULL DEFAULT 0;",
    );

    Ok(pool)
}

#[cfg(test)]
pub fn test_pool() -> DbPool {
    let pool = PoolConfig::new(":memory:")
        .create_pool(Runtime::Tokio1)
        .expect("Failed to create test pool");
    let conn = std::thread::spawn({
        let pool = pool.clone();
        move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let obj = pool.get().await.unwrap();
                let conn = obj.lock().unwrap();
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
                        rollback_on_failure INTEGER NOT NULL DEFAULT 0,
                        notify_events INTEGER NOT NULL DEFAULT 0
                    );
                    CREATE TABLE IF NOT EXISTS settings (
                        key TEXT PRIMARY KEY, value TEXT NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS container_updating (
                        container TEXT PRIMARY KEY
                    );",
                )
                .expect("Failed to create test schema");
            });
        }
    });
    conn.join().unwrap();
    pool
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
            e.duration_ms as i64,
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
            duration_ms: row.get::<_, i64>(6)? as u64,
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
            entry.duration_ms as i64,
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
        "SELECT container, action, cleanup_old_image, rollback_on_failure, notify_events FROM update_policies",
    )?;
    let rows = stmt.query_map([], |row| {
        let action_str: String = row.get(1)?;
        Ok(UpdatePolicy {
            container: row.get(0)?,
            action: action_str.parse().unwrap_or(UpdateAction::None),
            cleanup_old_image: row.get::<_, i32>(2)? != 0,
            rollback_on_failure: row.get::<_, i32>(3)? != 0,
            notify_events: row.get::<_, i32>(4)? != 0,
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
        "INSERT OR REPLACE INTO update_policies (container, action, cleanup_old_image, rollback_on_failure, notify_events)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            policy.container,
            policy.action.to_string(),
            policy.cleanup_old_image as i32,
            policy.rollback_on_failure as i32,
            policy.notify_events as i32,
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
                rollback_on_failure INTEGER NOT NULL DEFAULT 0,
                notify_events INTEGER NOT NULL DEFAULT 0
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
        };
        save_update_policy(&conn, &policy).unwrap();
        delete_update_policy(&conn, "nginx").unwrap();
        let loaded = load_update_policies(&conn).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_settings_save_load() {
        let conn = test_conn();
        let mut settings = Settings::default();
        settings.auto_update_enabled = Some(true);
        settings.auto_update_interval_hours = Some(12);
        settings.telegram_token = Some("bot123".into());
        settings.telegram_chat_id = Some("chat456".into());
        save_settings(&conn, &settings).unwrap();
        let loaded = load_settings(&conn).unwrap();
        assert_eq!(loaded.auto_update_enabled, Some(true));
        assert_eq!(loaded.auto_update_interval_hours, Some(12));
        assert_eq!(loaded.telegram_token.as_deref(), Some("bot123"));
    }

    #[test]
    fn test_load_settings_defaults() {
        let conn = test_conn();
        let settings = load_settings(&conn).unwrap();
        assert_eq!(settings.auto_update_enabled, None);
    }
}
