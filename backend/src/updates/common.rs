use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use chrono::Local;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[allow(dead_code)]
pub async fn notify_update_complete(
    notif_tx: &broadcast::Sender<NotifEvent>,
    settings: &Arc<Mutex<Settings>>,
    container: &str,
    status: &str,
) {
    let ts = Local::now().format("%H:%M:%S").to_string();
    let _ = notif_tx.send(NotifEvent {
        container: container.to_string(),
        status: format!("{} ✅", status),
        timestamp: ts,
    });
    notify_all(settings, container, &format!("✅ {}", status)).await;
}

#[allow(dead_code)]
pub async fn set_updating(db_pool: &DbPool, name: &str) {
    // Best-effort: marca un flag transitorio; si la BD falla solo se registra.
    match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => {
                if let Err(e) = crate::db::set_updating(&guard, name) {
                    tracing::warn!("set_updating: no se pudo marcar '{name}': {e}");
                }
            }
            Err(e) => tracing::warn!("set_updating: mutex de DB envenenado: {e}"),
        },
        Err(e) => tracing::warn!("set_updating: pool de DB no disponible: {e}"),
    }
}

#[allow(dead_code)]
pub async fn mark_update_done(db_pool: &DbPool, name: &str) {
    // Best-effort: limpieza de flags de estado; si la BD falla solo se registra.
    match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => {
                if let Err(e) = crate::db::clear_updating(&guard, name) {
                    tracing::warn!(
                        "mark_update_done: no se pudo limpiar updating de '{name}': {e}"
                    );
                }
                if let Err(e) = crate::db::update_container_has_update(&guard, name, false) {
                    tracing::warn!(
                        "mark_update_done: no se pudo limpiar has_update de '{name}': {e}"
                    );
                }
            }
            Err(e) => tracing::warn!("mark_update_done: mutex de DB envenenado: {e}"),
        },
        Err(e) => tracing::warn!("mark_update_done: pool de DB no disponible: {e}"),
    }
}

#[allow(dead_code)]
pub async fn clear_updating(db_pool: &DbPool, name: &str) {
    // Best-effort: limpieza de flag transitorio; si la BD falla solo se registra.
    match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => {
                if let Err(e) = crate::db::clear_updating(&guard, name) {
                    tracing::warn!("clear_updating: no se pudo limpiar '{name}': {e}");
                }
            }
            Err(e) => tracing::warn!("clear_updating: mutex de DB envenenado: {e}"),
        },
        Err(e) => tracing::warn!("clear_updating: pool de DB no disponible: {e}"),
    }
}

/// Compares local and remote digests to determine whether an image update
/// is needed.
///
/// Returns `true` when:
/// - `local_digest` is empty (first run / unknown state)
/// - `short_digest(local_digest) != short_digest(remote_digest)` (different content)
///
/// Returns `false` when:
/// - `short_digest(local_digest) == short_digest(remote_digest)` (same content)
///
/// The comparison uses [`short_digest`] (first 12 chars of the hex payload)
/// so that config-digest comparisons work correctly even when the manifest
/// digest differs from the config digest.
#[allow(dead_code)]
pub fn needs_update(local_digest: &str, remote_digest: &str) -> bool {
    if local_digest.is_empty() {
        return true;
    }
    crate::updates::digest::short_digest(local_digest)
        != crate::updates::digest::short_digest(remote_digest)
}

#[cfg(test)]
mod tests {
    use super::needs_update;
    /// Helper to build a full `sha256:<hex>` digest from a 12-char short value.
    #[expect(dead_code)]
    fn full_digest(short: &str) -> String {
        format!("sha256:{:<064}", short)
    }

    // ------------------------------------------------------------------
    // Scenario 1: same full digests → false
    // ------------------------------------------------------------------
    #[test]
    fn test_needs_update_same_digest() {
        let d = "sha256:aaaaaaaaaaaabbbbbbbbbbbbccccccccccddddddddddeeeeeeeeeeeffffffffff";
        assert!(
            !needs_update(d, d),
            "same full digest should NOT need update"
        );
    }

    // ------------------------------------------------------------------
    // Scenario 2: different digests → true
    // ------------------------------------------------------------------
    #[test]
    fn test_needs_update_different_digest() {
        let local = "sha256:aaaaaaaaaaaabbbbbbbbbbbbccccccccccddddddddddeeeeeeeeeeeffffffffff";
        let remote = "sha256:bbbbbbbbbbbbccccccccccddddddddddeeeeeeeeeeeffffffffffffffffffffffff";
        assert!(
            needs_update(local, remote),
            "different digests SHOULD need update"
        );
    }

    // ------------------------------------------------------------------
    // Scenario 3: empty local digest → true (first run)
    // ------------------------------------------------------------------
    #[test]
    fn test_needs_update_empty_local() {
        let remote = "sha256:aaaaaaaaaaaabbbbbbbbbbbbccccccccccddddddddddeeeeeeeeeeeffffffffff";
        assert!(
            needs_update("", remote),
            "empty local digest SHOULD need update (first run)"
        );
    }

    // ------------------------------------------------------------------
    // Scenario 4: manifest digest != config_digest, but
    //            last_remote_digest == config_digest → false
    //
    // This is the core bug scenario: the manifest digest
    // (Docker-Content-Digest header) may differ from the config digest
    // (body["config"]["digest"]), which Docker stores locally as the
    // ImageID. When we compare the *config* digest from the remote
    // against the *config* digest from local (image_id), they match →
    // no update needed, even though the manifest digest changed.
    // ------------------------------------------------------------------
    #[test]
    fn test_needs_update_manifest_vs_config() {
        // The config digest (image_id) stored locally by Docker.
        let local_image_id =
            "sha256:ccccccccccccddddddddddeeeeeeeeeeeffffffffffff00000000001111111111";

        // The config digest freshly retrieved from the remote registry.
        // This is what `last_remote_digest` stores — NOT the manifest digest.
        let remote_config_digest =
            "sha256:ccccccccccccddddddddddeeeeeeeeeeeffffffffffff00000000001111111111";

        // Even though the manifest digest might have changed (e.g. to
        // sha256:manifest_bbbb...), the config digest is the same →
        // the image content hasn't changed → no update needed.
        assert!(
            !needs_update(local_image_id, remote_config_digest),
            "same config digest should NOT need update even if manifest digest differs"
        );
    }

    // ------------------------------------------------------------------
    // Scenario 5: both empty → true
    // ------------------------------------------------------------------
    #[test]
    fn test_needs_update_both_empty() {
        assert!(
            needs_update("", ""),
            "both empty SHOULD need update (no known state)"
        );
    }
}
