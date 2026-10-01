use bollard::{
    container::{InspectContainerOptions, ListContainersOptions},
    Docker,
};
use chrono::Timelike;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::containers::pull_image;
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::notify_all;
use crate::updates::digest::RemoteDigest;
use crate::updates::handlers::{
    log_prune_result, prune_dangling_images, recreate_container, rollback_container,
    tag_backup_image, verify_container_healthy,
};

/// Worker que ejecuta revisiones de actualizaciones según el cron configurado.
/// Revisa todas las imágenes, marca las que tienen actualización pendiente en DB,
/// y aplica las políticas configuradas (pull, restart, prune).
#[allow(clippy::too_many_arguments)]
pub async fn update_check_worker(
    docker: Docker,
    settings: Arc<Mutex<Settings>>,
    update_policies: Arc<Mutex<Vec<UpdatePolicy>>>,
    update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: DbPool,
    update_in_progress: Arc<Mutex<HashSet<String>>>,
) {
    // Manifiestos degradados ya aplicados por contenedor. Evita reaplicar una
    // actualización degradada para el mismo manifest en ciclos sucesivos
    // (anti-bucle), sin persistir un manifest digest en `last_remote_digest`.
    let mut degraded_applied: HashMap<String, String> = HashMap::new();
    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        tick.tick().await;
        let s = settings.lock().await;
        let enabled = s.update_check_enabled.unwrap_or(false);
        let cron = s
            .update_check_cron
            .clone()
            .unwrap_or_else(|| "0 0 * * *".into()); // default: cada 24h a medianoche
        drop(s);

        if !enabled {
            continue;
        }
        let now = crate::timezone::now();
        let now_rounded = now
            .with_second(0)
            .and_then(|d| d.with_nanosecond(0))
            .unwrap_or(now);
        let expr = format!("0 {}", cron);
        let should_run = match expr.parse::<cron::Schedule>() {
            Ok(schedule) => schedule.includes(now_rounded),
            Err(e) => {
                tracing::warn!("update_check: cron inválido '{}': {}", cron, e);
                false
            }
        };
        if !should_run {
            continue;
        }

        tracing::info!(
            "update_check: revisando y aplicando actualizaciones (cron={})",
            cron
        );

        // 1. Obtener todos los contenedores
        let containers = docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();

        // Pre-resolve: bare digest images (sha256:...) only carry the digest
        // in `image`; inspect each container to recover the real image ref.
        let mut resolved_images: HashMap<String, String> = HashMap::new();
        for c in &containers {
            let image = c.image.as_deref().unwrap_or("");
            if image.starts_with("sha256:") {
                if let Some(cid) = &c.id {
                    if let Ok(inspect) = docker
                        .inspect_container(cid, None::<InspectContainerOptions>)
                        .await
                    {
                        let real = inspect
                            .config
                            .as_ref()
                            .and_then(|cfg| cfg.image.as_deref())
                            .unwrap_or("");
                        resolved_images.insert(cid.clone(), real.to_string());
                    }
                }
            }
        }

        // 1b. Cargar last_remote_digest de la DB para comparación correcta
        let last_remote_digest_map = load_last_remote_digest_map(&db_pool).await;

        // 2. Obtener políticas
        let policies = {
            let p = update_policies.lock().await;
            let map: HashMap<String, UpdatePolicy> = p
                .iter()
                .map(|pol| (pol.container.clone(), pol.clone()))
                .collect();
            let s = settings.lock().await;
            let default = (
                s.default_update_action
                    .clone()
                    .unwrap_or_else(|| "pull-restart".into()),
                s.default_cleanup_old_image.unwrap_or(false),
                s.default_rollback_on_failure.unwrap_or(false),
            );
            drop(s);
            (map, default)
        };
        let notify = {
            let s = settings.lock().await;
            s.update_check_notify.unwrap_or(false)
        };

        // 3. Revisar cada contenedor secuencialmente y aplicar políticas
        let check_interval_ms = {
            let s = settings.lock().await;
            s.check_interval_ms.unwrap_or(2000)
        };
        let mut updated_count = 0u32;
        // Cache de digests por imagen única dentro de este ciclo: evita
        // consultar el registry una vez por contenedor cuando la imagen se repite.
        let mut digest_cache: HashMap<String, Result<RemoteDigest, String>> = HashMap::new();

        for c in &containers {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| crate::models::strip_name(n))
                .unwrap_or_default();
            let raw_image = c.image.as_deref().unwrap_or("");
            let image_full = if raw_image.starts_with("sha256:") {
                c.id.as_ref()
                    .and_then(|cid| resolved_images.get(cid))
                    .map(|s| s.as_str())
                    .unwrap_or(raw_image)
                    .to_string()
            } else {
                raw_image.to_string()
            };
            let image_id = c.image_id.clone().unwrap_or_default();
            let cid = c.id.clone().unwrap_or_default();

            if image_full.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
                continue;
            }

            // Check remote digest for this container (deduplicado por imagen).
            let digest_result = resolve_digest_cached(&mut digest_cache, &image_full, || {
                let image = image_full.clone();
                let docker = docker.clone();
                async move {
                    crate::updates::digest::check_remote_digest_with_docker_detailed(
                        &image, &docker,
                    )
                    .await
                }
            })
            .await;

            let (has_update, old_digest, config_digest, manifest_digest, degraded) =
                match compute_update_decision(
                    &digest_result,
                    last_remote_digest_map.get(&name).map(|s| s.as_str()),
                    &image_id,
                    degraded_applied.get(&name).map(|s| s.as_str()),
                ) {
                    Some(decision) => (
                        decision.has_update,
                        decision.old_digest,
                        decision.config_digest,
                        decision.manifest_digest,
                        decision.degraded,
                    ),
                    None => {
                        let reason = digest_result.err().unwrap_or_default();
                        tracing::warn!(
                            "update_check [{}]: error resolviendo digest remoto de '{}': {} (se conserva el estado previo, NO se marca has_update=false)",
                            name,
                            image_full,
                            reason
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms))
                            .await;
                        continue;
                    }
                };

            let _ = sqlite_update_has_update(&db_pool, &name, has_update).await;
            if !has_update || name.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
                continue;
            }

            // Leer política para este contenedor
            let policy = match policies.0.get(&name) {
                Some(p) => p.clone(),
                None => UpdatePolicy {
                    container: name.clone(),
                    action: policies.1 .0.parse().unwrap_or(UpdateAction::PullRestart),
                    cleanup_old_image: policies.1 .1,
                    rollback_on_failure: policies.1 .2,
                    notify_events: false,
                },
            };
            if policy.action == UpdateAction::None {
                tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
                continue;
            }

            tracing::info!(
                "update_check: aplicando política {:?} a '{}'",
                policy.action,
                name
            );

            let start = std::time::Instant::now();
            match policy.action {
                UpdateAction::Pull => {
                    let _in_progress =
                        InProgressGuard::acquire(update_in_progress.clone(), name.clone()).await;
                    let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
                    if pull_image(&docker, &image_full, Some(&manifest_digest), pull_timeout).await
                    {
                        // Guard anti-bucle (mismo mecanismo que PullRestart):
                        // NO se persiste `last_remote_digest` en `Pull`; la
                        // semántica de no avanzar el digest se mantiene.
                        record_degraded_applied(
                            &mut degraded_applied,
                            &name,
                            degraded,
                            &manifest_digest,
                        );
                        _ = sqlite_append_update(
                            &db_pool,
                            &update_history,
                            &name,
                            &image_full,
                            &old_digest,
                            &config_digest,
                            "update-check-pull",
                            start.elapsed().as_millis() as u64,
                        )
                        .await;
                        if policy.cleanup_old_image {
                            let result = prune_dangling_images(&docker).await;
                            log_prune_result("scheduler-pull", &result);
                        }
                        updated_count += 1;
                    } else {
                        tracing::error!(
                            "update_check: pull FALLÓ para '{}' (image={})",
                            name,
                            image_full
                        );
                    }
                }
                UpdateAction::PullRestart => {
                    let _in_progress =
                        InProgressGuard::acquire(update_in_progress.clone(), name.clone()).await;
                    let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
                    let backup = if policy.rollback_on_failure {
                        tag_backup_image(&docker, &image_full).await
                    } else {
                        None
                    };
                    if pull_image(&docker, &image_full, Some(&manifest_digest), pull_timeout).await
                    {
                        match recreate_container(
                            &docker,
                            &name,
                            &cid,
                            &image_full,
                            Some(&manifest_digest),
                        )
                        .await
                        {
                            Ok(_) => {
                                if policy.rollback_on_failure
                                    && !verify_container_healthy(&docker, &name).await
                                {
                                    tracing::warn!("update_check: rollback '{}'", name);
                                    if let Some((backup_full, base, orig_tag)) = backup {
                                        rollback_container(
                                            &docker,
                                            &cid,
                                            &base,
                                            &orig_tag,
                                            &backup_full,
                                            &image_full,
                                        )
                                        .await;
                                    }
                                } else {
                                    if notify {
                                        notify_all(
                                            &settings,
                                            &name,
                                            "🔄 actualizado vía update-check",
                                        )
                                        .await;
                                    }
                                    let _ = sqlite_update_has_update(&db_pool, &name, false).await;
                                    // Persistir el digest real de la imagen en ejecución.
                                    // En modo degradado se inspecciona el contenedor en lugar
                                    // de guardar el manifest digest (que rompería la comparación).
                                    persist_last_remote_digest_after_recreate(
                                        &docker,
                                        &db_pool,
                                        &name,
                                        degraded,
                                        &config_digest,
                                    )
                                    .await;
                                    record_degraded_applied(
                                        &mut degraded_applied,
                                        &name,
                                        degraded,
                                        &manifest_digest,
                                    );
                                    _ = sqlite_append_update(
                                        &db_pool,
                                        &update_history,
                                        &name,
                                        &image_full,
                                        &old_digest,
                                        &config_digest,
                                        "update-check-restart",
                                        start.elapsed().as_millis() as u64,
                                    )
                                    .await;
                                    if policy.cleanup_old_image {
                                        let result = prune_dangling_images(&docker).await;
                                        log_prune_result("scheduler-restart", &result);
                                    }
                                    updated_count += 1;
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "update_check: recreate_container failed for '{}': {}",
                                    name,
                                    e
                                );
                            }
                        }
                    } else {
                        tracing::error!(
                            "update_check: pull FALLÓ para '{}' (image={})",
                            name,
                            image_full
                        );
                    }
                }
                UpdateAction::PullRestartStack => {
                    let _in_progress =
                        InProgressGuard::acquire(update_in_progress.clone(), name.clone()).await;
                    let pull_timeout = settings.lock().await.pull_timeout_secs.unwrap_or(600);
                    let backup = if policy.rollback_on_failure {
                        tag_backup_image(&docker, &image_full).await
                    } else {
                        None
                    };

                    // Recreación del servicio vía API Bollard (sin CLI docker
                    // ni compose file accesible dentro del contenedor).
                    let applied: Result<(), String> = async {
                        if !pull_image(&docker, &image_full, Some(&manifest_digest), pull_timeout)
                            .await
                        {
                            return Err("pull de la imagen falló".to_string());
                        }
                        recreate_container(
                            &docker,
                            &name,
                            &cid,
                            &image_full,
                            Some(&manifest_digest),
                        )
                        .await
                        .map_err(|e| format!("recreate del servicio falló: {}", e))
                    }
                    .await;

                    match applied {
                        Ok(()) => {
                            if policy.rollback_on_failure
                                && !verify_container_healthy(&docker, &name).await
                            {
                                tracing::warn!("update_check: rollback stack '{}'", name);
                                if let Some((backup_full, base, orig_tag)) = backup {
                                    rollback_container(
                                        &docker,
                                        &cid,
                                        &base,
                                        &orig_tag,
                                        &backup_full,
                                        &image_full,
                                    )
                                    .await;
                                }
                            } else {
                                if notify {
                                    notify_all(
                                        &settings,
                                        &name,
                                        "🔄 actualizado vía stack (update-check)",
                                    )
                                    .await;
                                }
                                let _ = sqlite_update_has_update(&db_pool, &name, false).await;
                                // Persistir el digest real del servicio recreado.
                                // En modo degradado se inspecciona el contenedor (el config
                                // digest degradado coincide con el manifest y no es fiable).
                                persist_last_remote_digest_after_recreate(
                                    &docker,
                                    &db_pool,
                                    &name,
                                    degraded,
                                    &config_digest,
                                )
                                .await;
                                record_degraded_applied(
                                    &mut degraded_applied,
                                    &name,
                                    degraded,
                                    &manifest_digest,
                                );
                                _ = sqlite_append_update(
                                    &db_pool,
                                    &update_history,
                                    &name,
                                    &image_full,
                                    &old_digest,
                                    &config_digest,
                                    "update-check-stack",
                                    start.elapsed().as_millis() as u64,
                                )
                                .await;
                                if policy.cleanup_old_image {
                                    let result = prune_dangling_images(&docker).await;
                                    log_prune_result("scheduler-stack", &result);
                                }
                                updated_count += 1;
                                tracing::info!(
                                    "update_check: '{}' actualizado vía stack (Bollard, image={})",
                                    name,
                                    image_full
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "update_check: PullRestartStack falló para '{}' (image={}): {}",
                                name,
                                image_full,
                                e
                            );
                        }
                    }
                }
                UpdateAction::None => {}
            }

            // Sleep between containers
            tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
        }

        // Safety-net prune at end
        let result = prune_dangling_images(&docker).await;
        log_prune_result("scheduler-safety-net", &result);

        // Actualizar last_check y next_check para todos los contenedores
        let last_check = crate::timezone::now_formatted();
        let next_check = crate::timezone::next_cron_time(&cron).unwrap_or_default();
        for c in &containers {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| crate::models::strip_name(n))
                .unwrap_or_default();
            if !name.is_empty() {
                if let Ok(obj) = db_pool.get().await {
                    match obj.lock() {
                        Ok(conn) => {
                            let _ = db::update_container_check_times(
                                &conn,
                                &name,
                                &last_check,
                                &next_check,
                            );
                        }
                        Err(e) => tracing::error!(
                            "update_check: mutex de DB envenenado al actualizar check times de '{}': {}",
                            name,
                            e
                        ),
                    }
                } else {
                    tracing::error!(
                        "update_check: no se pudo obtener conexión DB para check times de '{}'",
                        name
                    );
                }
            }
        }

        // Poda del guard degradado: elimina entradas de contenedores que ya no
        // existen (renombrado/recreación) para evitar crecimiento ilimitado.
        let present_names: HashSet<String> = containers
            .iter()
            .filter_map(|c| {
                c.names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| crate::models::strip_name(n))
            })
            .filter(|n| !n.is_empty())
            .collect();
        retain_present_containers(&mut degraded_applied, &present_names);

        // Persistir last_run_at en settings para que el endpoint API lo exponga
        {
            let mut s = settings.lock().await;
            s.update_check_last_run_at = Some(last_check.clone());
            let snapshot = s.clone();
            drop(s);
            if let Ok(conn) = db_pool.get().await {
                match conn.lock() {
                    Ok(guard) => {
                        let _ = db::save_settings(&guard, &snapshot);
                    }
                    Err(e) => tracing::error!(
                        "update_check: mutex de DB envenenado al guardar settings: {}",
                        e
                    ),
                }
            } else {
                tracing::error!(
                    "update_check: no se pudo obtener conexión DB para guardar settings"
                );
            }
        }

        tracing::info!(
            "update_check: {} contenedores revisados, {} actualizados/aplicados",
            containers.len(),
            updated_count
        );
    }
}

/// Decisión de actualización para un contenedor a partir del digest remoto.
#[derive(Debug, Clone, PartialEq, Eq)]
struct UpdateDecision {
    has_update: bool,
    old_digest: String,
    config_digest: String,
    manifest_digest: String,
    /// `true` si la resolución remota fue degradada (config == manifest).
    degraded: bool,
}

/// Digest que debe persistirse como `last_remote_digest` tras un recreate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PersistDigest<'a> {
    /// Config digest resuelto de forma fiable.
    Config(&'a str),
    /// Resolución degradada: el config digest no es fiable, hay que inspeccionar
    /// el contenedor en ejecución para recuperar el digest real.
    InspectRunning,
}

/// Decide qué digest persistir. Un resultado degradado nunca persiste el
/// `config_digest` (que reutiliza el manifest digest); se usa el fallback de
/// inspección del contenedor.
fn digest_to_persist(degraded: bool, config_digest: &str) -> PersistDigest<'_> {
    if degraded {
        PersistDigest::InspectRunning
    } else {
        PersistDigest::Config(config_digest)
    }
}

/// ¿Debe suprimirse una actualización degradada porque ya se aplicó ese mismo
/// manifest para el contenedor? Evita el re-update en cada ciclo cuando el
/// registry permanece degradado (`config != manifest` siempre daría "update").
fn degraded_update_suppressed(
    degraded: bool,
    manifest_digest: &str,
    already_applied: Option<&str>,
) -> bool {
    degraded && already_applied == Some(manifest_digest)
}

/// Registra (inserta/limpia) el manifest degradado aplicado a un contenedor.
/// Simétrico: resolución degradada → guarda el manifest; resolución fiable →
/// limpia la entrada. Es la única escritura del guard anti-bucle degradado.
fn record_degraded_applied(
    map: &mut HashMap<String, String>,
    name: &str,
    degraded: bool,
    manifest_digest: &str,
) {
    if degraded {
        map.insert(name.to_string(), manifest_digest.to_string());
    } else {
        map.remove(name);
    }
}

/// Poda del guard degradado: elimina las entradas de contenedores que ya no
/// están presentes en el ciclo (renombrado/recreación con nombre nuevo), para
/// evitar que el mapa crezca sin límite.
fn retain_present_containers(map: &mut HashMap<String, String>, present: &HashSet<String>) {
    map.retain(|name, _| present.contains(name));
}

/// Compute the update decision for a container from a resolved remote digest.
///
/// Returns `None` when the remote resolution failed: the caller MUST preserve
/// the previous `has_update` state rather than forcing it to `false` on a
/// transient registry/network failure.
fn compute_update_decision(
    resolved: &Result<RemoteDigest, String>,
    last_remote: Option<&str>,
    image_id: &str,
    degraded_applied_manifest: Option<&str>,
) -> Option<UpdateDecision> {
    let remote = match resolved {
        Ok(r) => r,
        Err(_) => return None,
    };
    let local_ref = last_remote.unwrap_or(image_id);
    let mut has_update = crate::updates::common::needs_update(local_ref, &remote.config_digest);
    // Anti-bucle: no volver a actualizar por una resolución degradada del mismo
    // manifest que ya se aplicó en un ciclo anterior.
    if degraded_update_suppressed(
        remote.degraded,
        &remote.manifest_digest,
        degraded_applied_manifest,
    ) {
        has_update = false;
    }
    Some(UpdateDecision {
        has_update,
        old_digest: local_ref.to_string(),
        config_digest: remote.config_digest.clone(),
        manifest_digest: remote.manifest_digest.clone(),
        degraded: remote.degraded,
    })
}

/// Resolve the remote digest of an image, reusing a per-cycle cache so the
/// registry is queried at most once per unique image reference.
///
/// `resolver` is only invoked on a cache miss; failures are cached too, so a
/// broken image is not retried for every container that shares it.
async fn resolve_digest_cached<F, Fut>(
    cache: &mut HashMap<String, Result<RemoteDigest, String>>,
    image_full: &str,
    resolver: F,
) -> Result<RemoteDigest, String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<RemoteDigest, String>>,
{
    if let Some(cached) = cache.get(image_full) {
        return cached.clone();
    }
    let result = resolver().await;
    cache.insert(image_full.to_string(), result.clone());
    result
}

/// Persist the `last_remote_digest` of a container after a successful recreate.
///
/// A degraded resolution MUST NOT be persisted as-is (it reuses the manifest
/// digest and would break the next comparison). In that case the running
/// container is inspected and its real image id is stored instead. Failures are
/// logged with `ERROR` instead of panicking.
async fn persist_last_remote_digest_after_recreate(
    docker: &Docker,
    db_pool: &DbPool,
    name: &str,
    degraded: bool,
    config_digest: &str,
) {
    let digest = match digest_to_persist(degraded, config_digest) {
        PersistDigest::Config(cfg) => cfg.to_string(),
        PersistDigest::InspectRunning => {
            tracing::info!(
                "update_check: resolución DEGRADED para '{}'; se inspecciona el contenedor para persistir el config digest real (no se persiste el manifest)",
                name
            );
            match docker
                .inspect_container(name, None::<InspectContainerOptions>)
                .await
            {
                Ok(inspect) => match inspect.image {
                    Some(image_id) => image_id,
                    None => {
                        tracing::error!(
                            "update_check: inspect_container('{}') sin image id; no se persiste last_remote_digest",
                            name
                        );
                        return;
                    }
                },
                Err(e) => {
                    tracing::error!(
                        "update_check: inspect_container('{}') falló: {}; no se persiste last_remote_digest",
                        name,
                        e
                    );
                    return;
                }
            }
        }
    };
    if let Ok(conn) = db_pool.get().await {
        match conn.lock() {
            Ok(guard) => {
                let _ = db::update_container_last_remote_digest(&guard, name, &digest);
            }
            Err(e) => tracing::error!(
                "update_check: mutex de DB envenenado al persistir digest de '{}': {}",
                name,
                e
            ),
        }
    } else {
        tracing::error!(
            "update_check: no se pudo obtener conexión DB para persistir digest de '{}'",
            name
        );
    }
}

/// RAII guard for `update_in_progress`.
///
/// Removes the container name from the set on drop — including on panic or
/// early return — so `state_worker` never silences notifications forever.
/// `tokio::sync::Mutex` cannot be locked synchronously; if the lock is held at
/// drop time the removal is deferred to a spawned task.
struct InProgressGuard {
    set: Arc<Mutex<HashSet<String>>>,
    name: String,
}

impl InProgressGuard {
    async fn acquire(set: Arc<Mutex<HashSet<String>>>, name: String) -> Self {
        set.lock().await.insert(name.clone());
        Self { set, name }
    }
}

impl Drop for InProgressGuard {
    fn drop(&mut self) {
        let set = Arc::clone(&self.set);
        let name = std::mem::take(&mut self.name);

        // Fast path: lock free → remove synchronously.
        {
            let lock = set.try_lock();
            if let Ok(mut guard) = lock {
                guard.remove(&name);
                return;
            }
        }

        // Lock held: defer removal to a spawned task (cannot await in Drop).
        let deferred = Arc::clone(&set);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                deferred.lock().await.remove(&name);
            });
        } else {
            tracing::error!(
                "update_in_progress: no hay runtime Tokio para limpiar '{}'",
                name
            );
        }
    }
}

/// Persistir has_update en DB (helper)
async fn sqlite_update_has_update(db_pool: &DbPool, name: &str, has_update: bool) {
    match db_pool.get().await {
        Ok(obj) => match obj.lock() {
            Ok(conn) => {
                let _ = db::update_container_has_update(&conn, name, has_update);
            }
            Err(e) => tracing::error!(
                "update_check: mutex de DB envenenado al persistir has_update de '{}': {}",
                name,
                e
            ),
        },
        Err(e) => tracing::error!(
            "update_check: no se pudo obtener conexión DB para has_update de '{}': {}",
            name,
            e
        ),
    }
}

/// Append a update history (helper)
#[allow(clippy::too_many_arguments)]
async fn sqlite_append_update(
    db_pool: &DbPool,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    name: &str,
    image: &str,
    old_digest: &str,
    new_digest: &str,
    status: &str,
    duration_ms: u64,
) {
    let entry = UpdateHistoryEntry {
        container: name.to_string(),
        image: image.to_string(),
        old_digest: old_digest.to_string(),
        new_digest: new_digest.to_string(),
        timestamp: crate::timezone::now_formatted(),
        status: status.to_string(),
        duration_ms,
    };
    // Publicar en memoria y soltar el guard ANTES del await de DB, para no
    // retener el mutex del historial durante la E/S.
    {
        let mut hist = update_history.lock().await;
        hist.push(entry.clone());
    }
    match db_pool.get().await {
        Ok(obj) => match obj.lock() {
            Ok(conn) => {
                let _ = db::append_update_history(&conn, &entry);
            }
            Err(e) => tracing::error!(
                "update_check: mutex de DB envenenado al anexar historial de '{}': {}",
                name,
                e
            ),
        },
        Err(e) => tracing::error!(
            "update_check: no se pudo obtener conexión DB para historial de '{}': {}",
            name,
            e
        ),
    }
}

/// Carga el mapa de nombre → last_remote_digest desde la DB.
/// Se usa para comparar contra el digest remoto actual y evitar
/// re-descargar imágenes que no han cambiado.
async fn load_last_remote_digest_map(db_pool: &DbPool) -> HashMap<String, String> {
    let conn = match db_pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                "update_check: no se pudo obtener conexión DB para cargar last_remote_digest: {}",
                e
            );
            return HashMap::new();
        }
    };
    let guard = match conn.lock() {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(
                "update_check: mutex de DB envenenado al cargar last_remote_digest: {}",
                e
            );
            return HashMap::new();
        }
    };
    let mut stmt = match guard
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    fn match_cron(cron: &str, dt: &chrono::DateTime<Local>) -> bool {
        let expr = format!("0 {}", cron);
        match expr.parse::<cron::Schedule>() {
            Ok(schedule) => schedule.includes(dt.to_utc()),
            Err(e) => {
                tracing::warn!("Invalid cron expression '{}': {}", cron, e);
                false
            }
        }
    }

    #[test]
    fn test_match_cron_every_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("* * * * *", &dt));
    }

    #[test]
    fn test_match_cron_specific_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 30, 0).unwrap();
        assert!(match_cron("30 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_wrong_minute() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 15, 0).unwrap();
        assert!(!match_cron("0 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_invalid_expression() {
        let dt = Local::now();
        assert!(!match_cron("invalid", &dt));
    }

    #[test]
    fn test_match_cron_empty() {
        let dt = Local::now();
        assert!(!match_cron("", &dt));
    }

    #[test]
    fn test_match_cron_every_5_minutes_first() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("*/5 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_every_5_minutes_fifth() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 5, 0).unwrap();
        assert!(match_cron("*/5 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_every_5_minutes_wrong() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 3, 0).unwrap();
        assert!(!match_cron("*/5 * * * *", &dt));
    }

    #[test]
    fn test_match_cron_specific_hour() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 3, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        assert!(match_cron("0 3 * * *", &dt));
    }

    #[test]
    fn test_match_cron_wrong_hour() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 4, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        assert!(!match_cron("0 3 * * *", &dt));
    }

    #[test]
    fn test_match_cron_daily_at_midnight() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        assert!(match_cron("0 0 * * *", &dt));
    }

    #[test]
    fn test_match_cron_range_hours() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        assert!(match_cron("0 9-17 * * *", &dt));
    }

    #[test]
    fn test_match_cron_outside_range_hours() {
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 20, 0, 0).unwrap();
        assert!(!match_cron("0 9-17 * * *", &dt));
    }

    #[test]
    fn test_match_cron_weekly() {
        let dt_utc = chrono::Utc.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap();
        let dt: chrono::DateTime<Local> = dt_utc.with_timezone(&Local);
        let _ = match_cron("0 0 * * 0", &dt);
    }

    #[test]
    fn test_match_cron_not_weekly() {
        let dt = Local.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let _ = match_cron("0 0 * * 0", &dt);
    }

    // ── Deduplicación por imagen (update-check-robustness) ──

    fn remote(manifest: &str, config: &str, tag: &str, degraded: bool) -> RemoteDigest {
        RemoteDigest {
            manifest_digest: manifest.to_string(),
            config_digest: config.to_string(),
            tag: tag.to_string(),
            degraded,
        }
    }

    #[tokio::test]
    async fn test_resolve_digest_cached_dedups_same_image() {
        let mut cache: HashMap<String, Result<RemoteDigest, String>> = HashMap::new();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        for _ in 0..3 {
            let calls = calls.clone();
            let result = resolve_digest_cached(&mut cache, "wordpress:fpm-alpine", || async move {
                calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(remote(
                    "sha256:manifest",
                    "sha256:config",
                    "fpm-alpine",
                    false,
                ))
            })
            .await;
            assert_eq!(result.unwrap().config_digest, "sha256:config");
        }
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "la imagen compartida debe resolverse una sola vez"
        );
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_resolve_digest_cached_distinct_images_resolve_each() {
        let mut cache: HashMap<String, Result<RemoteDigest, String>> = HashMap::new();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        for key in ["postgres:17", "postgres:18-alpine"] {
            let calls = calls.clone();
            let _ = resolve_digest_cached(&mut cache, key, || async move {
                calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(remote("m", "c", "t", false))
            })
            .await;
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(cache.len(), 2);
    }

    #[tokio::test]
    async fn test_resolve_digest_cached_caches_errors_too() {
        let mut cache: HashMap<String, Result<RemoteDigest, String>> = HashMap::new();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        for _ in 0..2 {
            let calls = calls.clone();
            let result: Result<RemoteDigest, String> =
                resolve_digest_cached(&mut cache, "broken:image", || async move {
                    calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err("registry down".to_string())
                })
                .await;
            assert!(result.is_err());
        }
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "un fallo también se cachea dentro del mismo ciclo"
        );
    }

    // ── Errores que no pisan estado ──

    #[test]
    fn test_compute_update_decision_error_yields_none() {
        let err: Result<RemoteDigest, String> = Err("registry unreachable".into());
        assert!(
            compute_update_decision(&err, Some("sha256:old"), "sha256:img", None).is_none(),
            "un error de resolución no debe producir decisión (no pisar estado)"
        );
    }

    #[test]
    fn test_compute_update_decision_same_config_no_update() {
        let ok: Result<RemoteDigest, String> =
            Ok(remote("sha256:manifest", "sha256:config", "latest", false));
        let d = compute_update_decision(&ok, Some("sha256:config"), "sha256:img", None).unwrap();
        assert!(!d.has_update);
        assert_eq!(d.old_digest, "sha256:config");
        assert_eq!(d.config_digest, "sha256:config");
        assert_eq!(d.manifest_digest, "sha256:manifest");
        assert!(!d.degraded);
    }

    #[test]
    fn test_compute_update_decision_first_run_uses_image_id() {
        let ok: Result<RemoteDigest, String> = Ok(remote(
            "sha256:manifest",
            "sha256:newconfig",
            "latest",
            false,
        ));
        let d = compute_update_decision(&ok, None, "sha256:imgid", None).unwrap();
        assert!(d.has_update);
        assert_eq!(d.old_digest, "sha256:imgid");
        assert_eq!(d.config_digest, "sha256:newconfig");
    }

    #[test]
    fn test_compute_update_decision_degraded_suppressed_when_already_applied() {
        let ok: Result<RemoteDigest, String> = Ok(remote("sha256:M", "sha256:M", "latest", true));
        // Mismo manifest ya aplicado en modo degradado → no re-update (anti-bucle).
        let suppressed =
            compute_update_decision(&ok, Some("sha256:C"), "sha256:img", Some("sha256:M")).unwrap();
        assert!(!suppressed.has_update);
        assert!(suppressed.degraded);
        // Manifest distinto → sí se permite actualizar.
        let allowed =
            compute_update_decision(&ok, Some("sha256:C"), "sha256:img", Some("sha256:M2"))
                .unwrap();
        assert!(allowed.has_update);
    }

    // ── Decisión de persistencia (degradado → inspeccionar contenedor) ──

    #[test]
    fn test_digest_to_persist_normal_uses_config() {
        assert_eq!(
            digest_to_persist(false, "sha256:config"),
            PersistDigest::Config("sha256:config")
        );
    }

    #[test]
    fn test_digest_to_persist_degraded_uses_inspect_fallback() {
        // El config digest degradado (== manifest) NO debe persistirse.
        assert_eq!(
            digest_to_persist(true, "sha256:manifest"),
            PersistDigest::InspectRunning
        );
    }

    #[test]
    fn test_degraded_update_suppressed_helper() {
        assert!(degraded_update_suppressed(
            true,
            "sha256:M",
            Some("sha256:M")
        ));
        assert!(!degraded_update_suppressed(
            true,
            "sha256:M2",
            Some("sha256:M")
        ));
        assert!(!degraded_update_suppressed(
            false,
            "sha256:M",
            Some("sha256:M")
        ));
        assert!(!degraded_update_suppressed(true, "sha256:M", None));
    }

    // ── Guard degradado: registro y poda (scheduler-reliability) ──

    #[test]
    fn test_record_degraded_applied_inserts_on_degraded() {
        let mut map = HashMap::new();
        record_degraded_applied(&mut map, "app", true, "sha256:M");
        assert_eq!(map.get("app").map(|s| s.as_str()), Some("sha256:M"));
    }

    #[test]
    fn test_record_degraded_applied_removes_on_reliable() {
        let mut map = HashMap::new();
        record_degraded_applied(&mut map, "app", true, "sha256:M");
        record_degraded_applied(&mut map, "app", false, "sha256:M");
        assert!(
            !map.contains_key("app"),
            "una resolución fiable limpia la entrada"
        );
    }

    #[test]
    fn test_record_degraded_applied_is_idempotent() {
        let mut map = HashMap::new();
        record_degraded_applied(&mut map, "app", true, "sha256:M");
        record_degraded_applied(&mut map, "app", true, "sha256:M");
        assert_eq!(map.len(), 1, "no debe duplicar entradas");
        assert_eq!(map.get("app").map(|s| s.as_str()), Some("sha256:M"));
    }

    #[test]
    fn test_record_degraded_applied_scoped_per_container() {
        let mut map = HashMap::new();
        record_degraded_applied(&mut map, "a", true, "sha256:A");
        record_degraded_applied(&mut map, "b", true, "sha256:B");
        record_degraded_applied(&mut map, "a", false, "sha256:A");
        assert!(!map.contains_key("a"));
        assert_eq!(map.get("b").map(|s| s.as_str()), Some("sha256:B"));
    }

    /// Valor real: tras registrar un pull degradado, la siguiente decisión
    /// (`compute_update_decision`) suprime el mismo manifest; un manifest nuevo
    /// no se suprime.
    #[test]
    fn test_recorded_degraded_manifest_suppresses_next_decision() {
        let mut map = HashMap::new();
        record_degraded_applied(&mut map, "app", true, "sha256:M");
        let same: Result<RemoteDigest, String> = Ok(remote("sha256:M", "sha256:M", "latest", true));
        let d = compute_update_decision(
            &same,
            Some("sha256:C"),
            "sha256:img",
            map.get("app").map(|s| s.as_str()),
        )
        .unwrap();
        assert!(
            !d.has_update,
            "el manifest degradado ya aplicado no vuelve a actualizar"
        );
        let new: Result<RemoteDigest, String> =
            Ok(remote("sha256:M2", "sha256:M2", "latest", true));
        let d2 = compute_update_decision(
            &new,
            Some("sha256:C"),
            "sha256:img",
            map.get("app").map(|s| s.as_str()),
        )
        .unwrap();
        assert!(d2.has_update, "un manifest degradado nuevo no se suprime");
    }

    // ── Poda del guard degradado (scheduler-reliability) ──

    #[test]
    fn test_retain_present_containers_removes_stale() {
        let mut map = HashMap::new();
        map.insert("keep".to_string(), "sha256:A".to_string());
        map.insert("stale".to_string(), "sha256:B".to_string());
        let present: HashSet<String> = std::iter::once("keep".to_string()).collect();
        retain_present_containers(&mut map, &present);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("keep").map(|s| s.as_str()), Some("sha256:A"));
        assert!(!map.contains_key("stale"));
    }

    #[test]
    fn test_retain_present_containers_keeps_all_present() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), "sha256:A".to_string());
        let present: HashSet<String> = std::iter::once("a".to_string()).collect();
        retain_present_containers(&mut map, &present);
        assert_eq!(map.len(), 1);
    }

    /// Test de forma: la fuente del worker no debe paniquear sobre el pool ni
    /// sobre los locks de conexión. Los patrones se componen en runtime para
    /// que el literal prohibido no aparezca en el propio archivo.
    #[test]
    fn test_no_unwrap_on_pool_or_lock_in_scheduler() {
        let src = include_str!("scheduler.rs");
        let pool_unwrap = concat!("db_pool.get().await.", "unwrap()");
        let lock_unwrap = concat!("lock().", "unwrap()");
        assert!(
            !src.contains(pool_unwrap),
            "scheduler.rs no debe paniquear al obtener una conexión del pool"
        );
        assert!(
            !src.contains(lock_unwrap),
            "scheduler.rs no debe paniquear sobre el mutex de conexión"
        );
    }
}
