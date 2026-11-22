use axum::{
    extract::{Path, State},
    response::Json,
};
use bollard::{
    container::{
        Config, CreateContainerOptions, InspectContainerOptions, ListContainersOptions,
        NetworkingConfig, RemoveContainerOptions, RenameContainerOptions, RestartContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    image::PruneImagesOptions,
    Docker,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, Notify};

use crate::containers::{
    fetch_containers, find_container_by_name, pull_image, stop_container, verify_stable_running,
};
use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::notifications::{notify_all, notify_all_level};
use crate::state::{RolledBackGuard, TerminalFailed};
use crate::updates::digest::check_remote_digest_with_docker;
use crate::updates::health_guard::{
    decide_after_verify, effective_health_window, effective_max_attempts, guard_history_status,
    guard_notify_level, may_remove_old_image, outcome_after_post_rollback_verify,
    rollback_reference, GuardAction, GuardOutcome, VerifyResult,
};
use bollard::models::ImagePruneResponse;

/// Cache progress for the long-polling state endpoint.
/// Updates the BatchProgress struct and notifies waiting handlers.
#[allow(clippy::too_many_arguments)]
async fn update_progress(
    progress_cache: &Arc<Mutex<BatchProgress>>,
    state_notify: &Arc<Notify>,
    progress_tx: &broadcast::Sender<UpdateProgress>,
    total: u32,
    checked: u32,
    updated: u32,
    errors: u32,
    checking: String,
) {
    let mut cache = progress_cache.lock().await;
    cache.total = total;
    cache.checked = checked;
    cache.updated = updated;
    cache.errors = errors;
    cache.checking = checking;
    let checking = cache.checking.clone();
    tracing::info!(
        "[update_progress] total={} checked={} updated={} errors={} checking={}",
        cache.total,
        cache.checked,
        cache.updated,
        cache.errors,
        cache.checking
    );
    drop(cache);
    state_notify.notify_waiters();
    // Expose progress on the SSE stream (`update-progress` event).
    let _ = progress_tx.send(UpdateProgress {
        container: checking,
        status: "checking".into(),
        done: false,
        error: None,
        total,
        checked,
        updated,
        errors,
        ..Default::default()
    });
}

/// Image reference written to `Config.Image` when recreating a container.
/// Always the clean `repo:tag` form, never a `repo:tag@sha256:...` pin.
pub(crate) fn recreate_image_ref(image_full: &str) -> String {
    crate::updates::digest::tag_ref(image_full).to_string()
}

/// Whether the initial `last_remote_digest` should be seeded for a container.
/// Seed only when no digest is stored yet and there is no pending update, so
/// the next cycle compares the running image against the remote.
fn should_seed_last_remote_digest(has_stored: bool, has_update: bool) -> bool {
    !has_stored && !has_update
}

/// Whether a successful policy action means the container was actually
/// recreated (so the freshly pulled image is now in use). Only then may
/// `has_update` be cleared and `last_remote_digest` advance.
///
/// `None` and `Pull` update the local image cache but leave the running
/// container on its old image, so they MUST NOT advance the digest.
pub(crate) fn should_advance_digest(action: &UpdateAction) -> bool {
    matches!(
        action,
        UpdateAction::PullRestart | UpdateAction::PullRestartStack
    )
}

/// Whether a `Pull` action can skip the download because the remote config
/// digest is already the one present in the local image cache.
///
/// Returns `true` only for `UpdateAction::Pull` when both digests are present,
/// non-empty and equal. Recreate actions (`PullRestart`/`PullRestartStack`)
/// always pull because they redeploy the container.
pub(crate) fn should_skip_pull(
    action: &UpdateAction,
    remote: Option<&str>,
    last_pulled: Option<&str>,
) -> bool {
    let (Some(remote), Some(last_pulled)) = (remote, last_pulled) else {
        return false;
    };
    *action == UpdateAction::Pull && !remote.is_empty() && remote == last_pulled
}

/// Persist `last_pulled_digest` for a container after a successful pull.
/// Does nothing when the digest is missing/empty, the DB is unavailable, or the
/// value is already stored (avoids redundant writes).
async fn persist_last_pulled_digest(db_pool: &DbPool, name: &str, digest: Option<&str>) {
    let Some(digest) = digest.filter(|d| !d.is_empty()) else {
        return;
    };
    if let Ok(conn) = db_pool.get().await {
        match conn.lock() {
            Ok(guard) => {
                if db::get_container_last_pulled_digest(&guard, name).as_deref() == Some(digest) {
                    return;
                }
                if let Err(e) = db::update_container_last_pulled_digest(&guard, name, digest) {
                    tracing::error!(
                        "persist_last_pulled_digest: error almacenando '{}': {}",
                        name,
                        e
                    );
                }
            }
            Err(e) => tracing::error!(
                "persist_last_pulled_digest: mutex envenenado para '{}': {}",
                name,
                e
            ),
        }
    } else {
        tracing::error!(
            "persist_last_pulled_digest: no se pudo obtener conexión DB para '{}'",
            name
        );
    }
}

/// Recreate a container by stopping, backing up, inspecting, creating with the
/// given image reference, starting, and cleaning up the backup. On start
/// failure, attempts an inline rollback (restore the backup container).
///
/// This is the **single** recreation mechanism: it serves both the forward
/// update (`repo:tag`) and the guard rollback (exact `sha256:...` digest).
///
/// `_digest` is accepted for call-site symmetry but is no longer used to build
/// `Config.Image`: the container is always recreated by the clean reference
/// passed in `image_full` (the pull-by-digest + retag step already points
/// `repo:tag` at the freshly downloaded content).
pub(crate) async fn recreate_container(
    docker: &Docker,
    name: &str,
    cid: &str,
    image_full: &str,
    _digest: Option<&str>,
) -> Result<(), String> {
    // 1. Stop the old container
    docker
        .stop_container(cid, None::<StopContainerOptions>)
        .await
        .map_err(|e| format!("stop failed: {}", e))?;

    // 2. Rename old container as backup
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let backup_name = format!("{}-old-{}", name, ts);
    docker
        .rename_container(
            cid,
            RenameContainerOptions {
                name: backup_name.clone(),
            },
        )
        .await
        .map_err(|e| format!("rename failed: {}", e))?;

    // 3. Inspect old container to get config
    let inspect = docker
        .inspect_container(&backup_name, None::<InspectContainerOptions>)
        .await
        .map_err(|e| format!("inspect failed: {}", e))?;

    // 4. Build new config from old one, modifying the image reference.
    // Always use the clean `repo:tag` form — never persist the `@sha256` pin.
    let mut container_config = inspect.config.unwrap_or_default();
    container_config.image = Some(recreate_image_ref(image_full));

    // Convert to bollard::container::Config and preserve host/networking config
    let mut config: Config<String> = container_config.into();
    config.host_config = inspect.host_config.clone();

    // Preserve network connections (critical for compose stacks)
    if let Some(networks) = inspect
        .network_settings
        .as_ref()
        .and_then(|ns| ns.networks.clone())
    {
        if !networks.is_empty() {
            config.networking_config = Some(NetworkingConfig {
                endpoints_config: networks,
            });
        }
    }

    // 5. Create new container with same name
    let create_opts = CreateContainerOptions {
        name: name.to_string(),
        platform: None,
    };
    docker
        .create_container(Some(create_opts), config)
        .await
        .map_err(|e| format!("create failed: {}", e))?;

    // 6. Start new container
    if let Err(e) = docker
        .start_container::<String>(name, None::<StartContainerOptions<String>>)
        .await
    {
        // If start fails, attempt rollback: remove the new container,
        // restore backup name, restart old container
        tracing::warn!(
            "recreate_container: start failed for '{}', attempting rollback: {}",
            name,
            e
        );
        let _ = docker
            .remove_container(name, None::<RemoveContainerOptions>)
            .await;
        let _ = docker
            .rename_container(
                &backup_name,
                RenameContainerOptions {
                    name: name.to_string(),
                },
            )
            .await;
        let _ = docker
            .start_container::<String>(name, None::<StartContainerOptions<String>>)
            .await;
        return Err(format!("start failed, rollback attempted: {}", e));
    }

    // 7. Remove old backup container
    let _ = docker
        .remove_container(&backup_name, None::<RemoveContainerOptions>)
        .await;

    Ok(())
}

/// Resultado de la guarda de arranque aplicada a una política.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GuardRun {
    /// Desenlace final: `Updated`, `RolledBack` o `FailedStopped`.
    pub outcome: GuardOutcome,
    /// Nº real de intentos de verificación de arranque realizados.
    pub attempts: u32,
}

/// Resuelve los parámetros efectivos de la guarda para una política.
/// El override de la política gana sobre el valor global; el máximo se
/// normaliza a `>= 1` para no provocar bucles infinitos.
pub(crate) fn guard_params(policy: &UpdatePolicy, settings: &Settings) -> (u32, u64, u64) {
    (
        effective_max_attempts(
            policy.max_start_attempts,
            settings.update_max_start_attempts,
        )
        .max(1),
        settings.update_start_attempt_interval_secs,
        effective_health_window(
            policy.health_window_secs,
            settings.update_health_window_secs,
        ),
    )
}

/// Recrea el contenedor con el **digest exacto** de la imagen anterior.
///
/// Delega en [`recreate_container`] (mecanismo único de recreación) usando el
/// digest persistido como referencia de imagen: nunca un tag frágil
/// `rollback-<ts>`.
pub(crate) async fn rollback_to_digest(
    docker: &Docker,
    name: &str,
    old_digest: &str,
) -> Result<(), String> {
    let reference = rollback_reference(old_digest);
    if reference.is_empty() {
        return Err("no hay digest previo para el rollback".to_string());
    }
    let cid = docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
        .ok()
        .and_then(|r| r.id)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| format!("no se pudo inspeccionar '{}' para el rollback", name))?;
    recreate_container(docker, name, &cid, &reference, Some(&reference)).await
}

/// Ejecuta la guarda de arranque sano sobre un contenedor ya recreado con la
/// imagen nueva.
///
/// Flujo (D3/D4/D1/D12):
/// 1. Verifica `running` estable durante `window_secs`.
/// 2. Si no es sano, reintenta hasta `max_attempts` (esperando `interval_secs`).
/// 3. Si se agotan, hace rollback por digest y verifica la imagen anterior:
///    sana ⇒ [`GuardOutcome::RolledBack`]; no sana ⇒ para el contenedor ⇒
///    [`GuardOutcome::FailedStopped`].
///
/// Devuelve también el nº real de intentos para el historial.
///
/// # Errores
///
/// Un fallo de **inspección** de Docker (`verify_stable_running` ⇒ `Err`) es
/// inconcluso, no un «no arrancó sano» (design D12): se registra con `ERROR`,
/// se **omite el teardown** (ni rollback ni parada) y se propaga el error para
/// que el llamador no descarte la imagen anterior.
pub(crate) async fn run_start_guard(
    docker: &Docker,
    name: &str,
    old_digest: &str,
    max_attempts: u32,
    interval_secs: u64,
    window_secs: u64,
) -> Result<GuardRun, AppError> {
    let max_attempts = max_attempts.max(1);
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        let verify = verify_stable_running(docker, name, window_secs).await;
        match decide_after_verify(VerifyResult::from_result(&verify), attempts, max_attempts) {
            GuardAction::Accept => {
                return Ok(GuardRun {
                    outcome: GuardOutcome::Updated,
                    attempts,
                });
            }
            GuardAction::RetryStart => {
                tracing::warn!(
                    "update-guard [{}]: arranque no sano (intento {}/{}), reintentando",
                    name,
                    attempts,
                    max_attempts
                );
                if interval_secs > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
                }
                if let Err(e) = docker
                    .restart_container(name, None::<RestartContainerOptions>)
                    .await
                {
                    tracing::warn!("update-guard [{}]: restart falló: {}", name, e);
                }
            }
            GuardAction::Rollback => break,
            GuardAction::SkipTeardown => {
                // D12: error de inspección ≠ contenedor no sano. No se hace
                // rollback ni se para el contenedor, y la imagen anterior queda
                // intacta (el llamador no la descarta por este camino).
                let cause = verify
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "causa desconocida".to_string());
                tracing::error!(
                    "update-guard [{}]: fallo de inspección en el intento {}/{}: {} — se omite el teardown (ni rollback ni parada) y se conserva la imagen anterior",
                    name,
                    attempts,
                    max_attempts,
                    cause
                );
                return Err(AppError::Docker(format!(
                    "verify_stable_running '{}': inspección fallida en el intento {}: {}",
                    name, attempts, cause
                )));
            }
        }
    }

    tracing::warn!(
        "update-guard [{}]: {} intentos agotados, rollback a {}",
        name,
        attempts,
        crate::updates::digest::short_digest(old_digest)
    );

    let rolled_back = rollback_to_digest(docker, name, old_digest).await;
    if let Err(e) = &rolled_back {
        tracing::error!("update-guard [{}]: rollback falló: {}", name, e);
    }
    // La verificación de la imagen anterior sólo tiene sentido si el rollback
    // se aplicó; si no, el desenlace es «no sana» (rollback fallido).
    let post_verify = if rolled_back.is_ok() {
        verify_stable_running(docker, name, window_secs).await
    } else {
        Ok(false)
    };
    let post_result = VerifyResult::from_result(&post_verify);
    if post_result == VerifyResult::InspectionError {
        // D12: inconcluso ⇒ no se para el contenedor.
        if let Err(e) = &post_verify {
            tracing::error!(
                "update-guard [{}]: verificación post-rollback inconclusa (error de inspección): {} — no se detiene el contenedor",
                name,
                e
            );
        }
    }

    let outcome = outcome_after_post_rollback_verify(post_result);
    if outcome == GuardOutcome::FailedStopped {
        tracing::error!(
            "update-guard [{}]: la imagen anterior tampoco arranca sana; contenedor detenido",
            name
        );
        if let Err(e) = stop_container(docker, name).await {
            tracing::error!("update-guard [{}]: stop falló: {}", name, e);
        }
    } else {
        tracing::warn!(
            "update-guard [{}]: rollback sano (imagen anterior OK)",
            name
        );
    }

    Ok(GuardRun { outcome, attempts })
}

/// Recrea el contenedor con la imagen nueva y aplica la guarda de arranque.
///
/// Único punto de entrada compartido por el worker de updates y los flujos
/// manuales (`update_container_h`, `update_all_h`, `update_stack_h` y
/// `apply_single_policy`) — design D11:
///
/// 1. `recreate_container` con la referencia limpia `repo:tag`.
/// 2. Con `rollback_on_failure` activo, `run_start_guard` (verificación de la
///    ventana estable → reintentos → rollback por digest exacto → estado
///    terminal).
/// 3. Con `rollback_on_failure` inactivo se conserva el comportamiento previo
///    (sin verificación ampliada ni rollback), por decisión explícita de diseño
///    (spec `updates/update-check`).
///
/// El llamador MUST NOT descartar la imagen anterior salvo que el desenlace sea
/// [`GuardOutcome::Updated`] (ver [`may_remove_old_image`]).
///
/// # Errores
///
/// `Err` si la recreación falla o si la verificación queda inconclusa por un
/// fallo de inspección (D12: sin teardown).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_with_guard(
    docker: &Docker,
    name: &str,
    cid: &str,
    image_full: &str,
    manifest_digest: Option<&str>,
    old_digest: &str,
    max_attempts: u32,
    interval_secs: u64,
    window_secs: u64,
    rollback_on_failure: bool,
) -> Result<GuardRun, AppError> {
    recreate_container(docker, name, cid, image_full, manifest_digest)
        .await
        .map_err(AppError::Docker)?;

    if !rollback_on_failure {
        return Ok(GuardRun {
            outcome: GuardOutcome::Updated,
            attempts: 0,
        });
    }

    run_start_guard(
        docker,
        name,
        old_digest,
        max_attempts,
        interval_secs,
        window_secs,
    )
    .await
}

/// Añade una entrada al historial en memoria y la persiste en DB.
async fn push_history_entry(
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: &DbPool,
    entry: UpdateHistoryEntry,
) {
    {
        let mut hist = update_history.lock().await;
        hist.push(entry.clone());
    }
    // Best-effort: la entrada ya está en memoria; si la BD falla solo se registra.
    match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => {
                if let Err(e) = db::append_update_history(&guard, &entry) {
                    tracing::warn!(
                        "push_history_entry: no se pudo persistir historial de '{}': {e}",
                        entry.container
                    );
                }
            }
            Err(e) => tracing::warn!("push_history_entry: mutex de DB envenenado: {e}"),
        },
        Err(e) => tracing::warn!("push_history_entry: pool de DB no disponible: {e}"),
    }
}

/// Datos de un desenlace no exitoso de la guarda para el historial y la
/// notificación.
pub(crate) struct GuardOutcomeReport<'a> {
    pub name: &'a str,
    pub image: &'a str,
    pub old_digest: &'a str,
    /// Digest remoto (config) al que se intentó actualizar. Es el digest que
    /// alimenta el mapa anti-churn `rolled_back` (design D10).
    pub attempted_digest: &'a str,
    pub attempts: u32,
    pub duration_ms: u64,
    /// Mensaje de la notificación; el nivel lo fija [`guard_notify_level`].
    pub message: &'a str,
}

/// Registra en historial, notifica y aplica los efectos en memoria de un
/// desenlace **no exitoso** de la guarda (`rolled-back` / `failed-stopped`).
///
/// Unifica el bloque decisión→historial→notificación antes duplicado entre el
/// worker y los handlers manuales (design D11, hallazgo 8 del reviewer):
///
/// - `rolled-back` ⇒ alimenta el mapa anti-churn `rolled_back` con
///   `(name, attempted_digest)` para que el ciclo no reintente el mismo digest
///   (design D10).
/// - `failed-stopped` ⇒ marca el contenedor como terminal para que no se
///   reintente hasta intervención explícita (design D9).
///
/// NO elimina la imagen anterior: sólo [`may_remove_old_image`] autoriza esa
/// limpieza.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn record_guard_outcome(
    outcome: GuardOutcome,
    report: GuardOutcomeReport<'_>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: &DbPool,
    settings: &Arc<Mutex<Settings>>,
    terminal_failed: &TerminalFailed,
    rolled_back: &RolledBackGuard,
    notify: bool,
) {
    match outcome {
        GuardOutcome::RolledBack => {
            let mut map = rolled_back.0.lock().await;
            map.insert(report.name.to_string(), report.attempted_digest.to_string());
        }
        GuardOutcome::FailedStopped => {
            let mut set = terminal_failed.0.lock().await;
            set.insert(report.name.to_string());
        }
        // `Updated` no es un desenlace de fallo: no hay nada que registrar aquí.
        GuardOutcome::Updated => return,
    }

    push_history_entry(
        update_history,
        db_pool,
        UpdateHistoryEntry {
            container: report.name.to_string(),
            image: report.image.to_string(),
            old_digest: report.old_digest.to_string(),
            new_digest: report.attempted_digest.to_string(),
            timestamp: crate::timezone::now_formatted(),
            status: guard_history_status(outcome).to_string(),
            duration_ms: report.duration_ms,
            attempts: report.attempts,
        },
    )
    .await;

    if notify {
        notify_all_level(
            settings,
            report.name,
            report.message,
            guard_notify_level(outcome),
        )
        .await;
    }
}

/// Limpia la supresión (D9/D10) de un contenedor ante una intervención manual
/// del operador: sale del estado terminal `failed-stopped` (`terminal_failed`) y
/// reinicia el anti-churn de rollback (`rolled_back`), de modo que el contenedor
/// vuelve a ser candidato a update.
///
/// Es la operación inversa de [`record_guard_outcome`] y el punto único de
/// limpieza manual compartido por [`update_container_h`], [`update_all_h`] y
/// `stacks::update_stack_h`, para que los tres flujos sean idénticos (D9/D10).
///
/// NOTA: los contenedores `failed-stopped` están **parados** y por eso NO
/// aparecen en el listado de `update_all_h` (`docker_list_running`, `all: false`);
/// su reintento se realiza vía [`update_container_h`] (`POST /api/update/{name}`).
pub(crate) async fn clear_suppression(
    name: &str,
    terminal_failed: &TerminalFailed,
    rolled_back: &RolledBackGuard,
) {
    terminal_failed.0.lock().await.remove(name);
    rolled_back.0.lock().await.remove(name);
}

struct PendingUpdate {
    name: String,
    image_full: String,
    cid: String,
    image_id: String,
    remote_digest: Option<String>, // config digest (for comparison + DB storage)
    manifest_digest: Option<String>, // manifest digest (for pull + recreate)
    last_pulled_digest: Option<String>, // cached config digest (skip redundant pulls)
    compose_project: Option<String>,
}

pub async fn update_container_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
    State(terminal_failed): State<TerminalFailed>,
    State(rolled_back): State<RolledBackGuard>,
    Path(name): Path<String>,
) -> Result<Json<UpdateProgress>, AppError> {
    // Intervención explícita del operador (D9/D10): reanuda la gestión
    // automática del contenedor saliendo del estado terminal `failed-stopped` y
    // limpiando la supresión anti-churn tras un rollback.
    clear_suppression(&name, &terminal_failed, &rolled_back).await;

    let container = find_container_by_name(&docker, &name).await?;
    let image = container.image.as_deref().unwrap_or("");
    let cid = container.id.as_deref().unwrap_or("");

    if image.is_empty() {
        return Err(AppError::BadRequest("container has no image".into()));
    }

    // Verificar digest remoto antes de hacer pull
    let image_id = container.image_id.as_deref().unwrap_or("").to_string();
    let (needs_pull, remote_digest, manifest_digest) =
        match crate::updates::digest::check_remote_digest_with_docker(image, &docker).await {
            Ok((manifest, config, _)) => {
                // Use last_remote_digest from DB if available, fallback to image_id
                let last_remote = {
                    let conn = db_pool.get().await.map_err(AppError::from_pool)?;
                    let guard = conn.lock().map_err(AppError::from_pool)?;
                    crate::db::get_container_last_remote_digest(&guard, &name)
                        .unwrap_or_else(|| image_id.clone())
                };
                let needs = crate::updates::common::needs_update(&last_remote, &config);
                (needs, config, manifest)
            }
            Err(e) => {
                tracing::warn!(
                    "update_container [{}]: error verificando digest remoto: {}, NO se hará pull",
                    name,
                    e
                );
                return Err(AppError::Internal(format!(
                    "cannot check remote digest: {}",
                    e
                )));
            }
        };

    if !needs_pull {
        return Ok(Json(UpdateProgress {
            container: name,
            status: "already-up-to-date".into(),
            done: true,
            error: None,
            total: 0,
            checked: 0,
            updated: 0,
            errors: 0,
            ..Default::default()
        }));
    }

    let pull_timeout = settings.lock().await.effective_pull_timeout_secs();
    let start_time = std::time::Instant::now();
    tracing::info!(
        "update_container_h: descargando '{}' (timeout: {}s)",
        image,
        pull_timeout
    );
    if !pull_image(&docker, image, Some(&manifest_digest), pull_timeout).await {
        let entry = UpdateHistoryEntry {
            container: name.clone(),
            image: image.to_string(),
            old_digest: image_id.clone(),
            new_digest: remote_digest.clone(),
            timestamp: crate::timezone::now_formatted(),
            status: "error".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            attempts: 0,
        };
        // Best-effort: el historial es un efecto lateral; un fallo de BD NO debe
        // convertir un pull fallido en un 500 distinto ni abortar la respuesta.
        // NOTE: do NOT advance `last_remote_digest` here — the pull failed,
        // so the running container is still on the old image.
        push_history_entry(&update_history, &db_pool, entry).await;
        return Err(AppError::Internal("pull failed".into()));
    }
    // Record the digest of the image now present in the local cache.
    persist_last_pulled_digest(&db_pool, &name, Some(&remote_digest)).await;

    // Parámetros de la guarda para el flujo manual: settings globales (este
    // handler no resuelve política por contenedor).
    let (guard_max, guard_interval, guard_window, rollback_on_failure, notify) = {
        let s = settings.lock().await;
        (
            s.update_max_start_attempts.max(1),
            s.update_start_attempt_interval_secs,
            s.update_health_window_secs,
            s.default_rollback_on_failure.unwrap_or(false),
            s.update_check_notify.unwrap_or(false),
        )
    };

    // La recreación pasa por la guarda compartida (D11): la imagen anterior
    // sólo se descarta tras confirmar la nueva sana.
    match apply_with_guard(
        &docker,
        &name,
        cid,
        image,
        Some(&manifest_digest),
        &image_id,
        guard_max,
        guard_interval,
        guard_window,
        rollback_on_failure,
    )
    .await
    {
        Ok(guard) if may_remove_old_image(guard.outcome) => {
            tracing::info!(
                "update_container_h: '{}' reiniciado correctamente (intentos: {})",
                name,
                guard.attempts
            );
            notify_all(&settings, &name, "✅ actualizado y reiniciado").await;
            // La nueva imagen está confirmada sana: ahora sí se puede limpiar
            // la anterior.
            crate::containers::remove_old_image(&docker, &image_id).await;
            {
                let conn = db_pool.get().await.map_err(AppError::from_pool)?;
                let conn_lock = conn.lock().map_err(AppError::from_pool)?;
                let _ = db::update_container_has_update(&conn_lock, &name, false);
                let _ = db::update_container_last_remote_digest(&conn_lock, &name, &remote_digest);
            }
            push_history_entry(
                &update_history,
                &db_pool,
                UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.to_string(),
                    old_digest: image_id.clone(),
                    new_digest: remote_digest.clone(),
                    timestamp: crate::timezone::now_formatted(),
                    status: guard_history_status(guard.outcome).to_string(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    attempts: guard.attempts,
                },
            )
            .await;
            Ok(Json(UpdateProgress {
                container: name,
                status: "ok".into(),
                done: true,
                error: None,
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
                ..Default::default()
            }))
        }
        Ok(guard) => {
            // `rolled-back` / `failed-stopped`: historial + notificación +
            // efectos en memoria (anti-churn D10 / terminal D9) en el helper
            // compartido. La imagen anterior se conserva.
            record_guard_outcome(
                guard.outcome,
                GuardOutcomeReport {
                    name: &name,
                    image,
                    old_digest: &image_id,
                    attempted_digest: &remote_digest,
                    attempts: guard.attempts,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    message: guard_outcome_message(guard.outcome),
                },
                &update_history,
                &db_pool,
                &settings,
                &terminal_failed,
                &rolled_back,
                notify,
            )
            .await;
            Ok(Json(UpdateProgress {
                container: name,
                status: guard_history_status(guard.outcome).into(),
                done: true,
                error: None,
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
                ..Default::default()
            }))
        }
        Err(e) => {
            tracing::error!("update_container_h: error al reiniciar '{}': {}", name, e);
            push_history_entry(
                &update_history,
                &db_pool,
                UpdateHistoryEntry {
                    container: name.clone(),
                    image: image.to_string(),
                    old_digest: image_id.clone(),
                    new_digest: remote_digest.clone(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    attempts: 0,
                },
            )
            .await;
            Err(AppError::Docker(format!("restart: {}", e)))
        }
    }
}

/// Mensaje de notificación asociado a un desenlace no exitoso de la guarda.
pub(crate) fn guard_outcome_message(outcome: GuardOutcome) -> &'static str {
    match outcome {
        GuardOutcome::RolledBack => {
            "⚠️ actualización revertida (rollback): imagen anterior restaurada"
        }
        GuardOutcome::FailedStopped => {
            "⛔ fallo terminal: la imagen anterior tampoco arranca; contenedor detenido"
        }
        GuardOutcome::Updated => "✅ actualizado",
    }
}

pub async fn update_all_h(
    State(docker): State<Docker>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(db_pool): State<DbPool>,
    State(terminal_failed): State<TerminalFailed>,
    State(rolled_back): State<RolledBackGuard>,
) -> Result<Json<Vec<UpdateProgress>>, AppError> {
    let (guard_max, guard_interval, guard_window, rollback_on_failure, notify) = {
        let s = settings.lock().await;
        (
            s.update_max_start_attempts.max(1),
            s.update_start_attempt_interval_secs,
            s.update_health_window_secs,
            s.default_rollback_on_failure.unwrap_or(false),
            s.update_check_notify.unwrap_or(false),
        )
    };
    let mut results = vec![];
    for (name, image, cid, image_id) in crate::workers::docker_list_running(&docker).await {
        // Intervención manual (D9/D10): limpiar la supresión de cada contenedor
        // iterado antes de aplicar, igual que `update_container_h`. Los
        // contenedores `failed-stopped` están **parados** y por eso no entran en
        // este listado (`docker_list_running` usa `all: false`): su reintento se
        // hace vía `update_container_h` (`POST /api/update/{name}`).
        clear_suppression(&name, &terminal_failed, &rolled_back).await;
        // Verificar digest remoto antes de hacer pull
        let (needs_pull, remote_digest, manifest_digest) =
            match crate::updates::digest::check_remote_digest_with_docker(&image, &docker).await {
                Ok((manifest, config, _)) => {
                    let has_update = image_id.as_ref().is_none_or(|local_digest| {
                        let local_short = crate::updates::digest::short_digest(local_digest);
                        let remote_short = crate::updates::digest::short_digest(&config);
                        local_short != remote_short
                    });
                    (has_update, config, manifest)
                }
                Err(e) => {
                    tracing::warn!(
                        "update_all [{}]: error verificando digest remoto: {}, NO se hará pull",
                        name,
                        e
                    );
                    results.push(UpdateProgress {
                        container: name.clone(),
                        status: "error".into(),
                        done: true,
                        error: Some(format!("digest check failed: {}", e)),
                        total: 0,
                        checked: 0,
                        updated: 0,
                        errors: 0,
                        ..Default::default()
                    });
                    continue;
                }
            };

        if !needs_pull {
            results.push(UpdateProgress {
                container: name.clone(),
                status: "✅ ya actualizado".into(),
                done: true,
                error: None,
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
                ..Default::default()
            });
            continue;
        }

        let old_digest = image_id.as_deref().unwrap_or("").to_string();
        let pull_timeout = settings.lock().await.effective_pull_timeout_secs();
        let start_time = std::time::Instant::now();
        tracing::info!(
            "update_all_h: descargando '{}' (imagen: {}, timeout: {}s)",
            name,
            image,
            pull_timeout
        );
        if !pull_image(&docker, &image, Some(&manifest_digest), pull_timeout).await {
            tracing::error!("update_all_h: pull FALLÓ para '{}'", name);
            results.push(UpdateProgress {
                container: name.clone(),
                status: "error".into(),
                done: true,
                error: Some("pull failed".into()),
                total: 0,
                checked: 0,
                updated: 0,
                errors: 0,
                ..Default::default()
            });
            let entry = UpdateHistoryEntry {
                container: name.clone(),
                image: image.clone(),
                old_digest,
                new_digest: remote_digest.clone(),
                timestamp: crate::timezone::now_formatted(),
                status: "error".into(),
                duration_ms: start_time.elapsed().as_millis() as u64,
                attempts: 0,
            };
            // Best-effort: persistir el historial no debe abortar el batch y
            // perder el array de resultados ya acumulado en `results`.
            push_history_entry(&update_history, &db_pool, entry).await;
            continue;
        }
        tracing::info!("update_all_h: pull OK para '{}', reiniciando...", name);
        // Record the digest of the image now present in the local cache.
        persist_last_pulled_digest(&db_pool, &name, Some(&remote_digest)).await;
        // La recreación pasa por la guarda compartida (D11): la imagen anterior
        // sólo se descarta tras confirmar la nueva sana.
        match apply_with_guard(
            &docker,
            &name,
            &cid,
            &image,
            Some(&manifest_digest),
            &old_digest,
            guard_max,
            guard_interval,
            guard_window,
            rollback_on_failure,
        )
        .await
        {
            Ok(guard) if may_remove_old_image(guard.outcome) => {
                tracing::info!(
                    "update_all_h: contenedor '{}' recreado correctamente (intentos: {})",
                    name,
                    guard.attempts
                );
                notify_all(&settings, &name, "✅ actualizado").await;
                // Confirmada la nueva imagen sana: ahora sí se puede limpiar la
                // anterior.
                crate::containers::remove_old_image(&docker, &old_digest).await;
                // Best-effort: persistir flags/digest no altera el array de resultados.
                match db_pool.get().await {
                    Ok(conn) => match conn.lock() {
                        Ok(conn_lock) => {
                            if let Err(e) =
                                db::update_container_has_update(&conn_lock, &name, false)
                            {
                                tracing::warn!(
                                    "update_all_h: no se pudo limpiar has_update de '{}': {e}",
                                    name
                                );
                            }
                            // Recreate succeeded → the running image is now the remote
                            // config digest; advance `last_remote_digest`.
                            if let Err(e) = db::update_container_last_remote_digest(
                                &conn_lock,
                                &name,
                                &remote_digest,
                            ) {
                                tracing::warn!(
                                    "update_all_h: no se pudo persistir last_remote_digest de '{}': {e}",
                                    name
                                );
                            }
                        }
                        Err(e) => tracing::error!(
                            "update_all_h: mutex envenenado al persistir digest de '{}': {}",
                            name,
                            e
                        ),
                    },
                    Err(e) => tracing::warn!(
                        "update_all_h: pool de DB no disponible persistiendo digest de '{}': {e}",
                        name
                    ),
                }
                results.push(UpdateProgress {
                    container: name.clone(),
                    status: "ok".into(),
                    done: true,
                    error: None,
                    total: 0,
                    checked: 0,
                    updated: 0,
                    errors: 0,
                    ..Default::default()
                });
                push_history_entry(
                    &update_history,
                    &db_pool,
                    UpdateHistoryEntry {
                        container: name.clone(),
                        image: image.clone(),
                        old_digest,
                        new_digest: remote_digest.clone(),
                        timestamp: crate::timezone::now_formatted(),
                        status: guard_history_status(guard.outcome).to_string(),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                        attempts: guard.attempts,
                    },
                )
                .await;
            }
            Ok(guard) => {
                // `rolled-back` / `failed-stopped`: historial + notificación +
                // efectos en memoria (D9/D10) vía el helper compartido. La
                // imagen anterior se conserva.
                record_guard_outcome(
                    guard.outcome,
                    GuardOutcomeReport {
                        name: &name,
                        image: &image,
                        old_digest: &old_digest,
                        attempted_digest: &remote_digest,
                        attempts: guard.attempts,
                        duration_ms: start_time.elapsed().as_millis() as u64,
                        message: guard_outcome_message(guard.outcome),
                    },
                    &update_history,
                    &db_pool,
                    &settings,
                    &terminal_failed,
                    &rolled_back,
                    notify,
                )
                .await;
                results.push(UpdateProgress {
                    container: name,
                    status: guard_history_status(guard.outcome).into(),
                    done: true,
                    error: None,
                    total: 0,
                    checked: 0,
                    updated: 0,
                    errors: 0,
                    ..Default::default()
                });
            }
            Err(e) => {
                tracing::error!("update_all_h: error al reiniciar '{}': {}", name, e);
                push_history_entry(
                    &update_history,
                    &db_pool,
                    UpdateHistoryEntry {
                        container: name.clone(),
                        image: image.clone(),
                        old_digest: old_digest.clone(),
                        new_digest: remote_digest.clone(),
                        timestamp: crate::timezone::now_formatted(),
                        status: "error".into(),
                        duration_ms: start_time.elapsed().as_millis() as u64,
                        attempts: 0,
                    },
                )
                .await;

                results.push(UpdateProgress {
                    container: name,
                    status: "error".into(),
                    done: true,
                    error: Some(e.to_string()),
                    total: 0,
                    checked: 0,
                    updated: 0,
                    errors: 0,
                    ..Default::default()
                });
            }
        }
    }
    Ok(Json(results))
}

pub async fn check_update_h(
    State(docker): State<Docker>,
    State(db_pool): State<db::DbPool>,
    Path(name): Path<String>,
) -> Result<Json<VersionCompare>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let image_full = container.image.as_deref().unwrap_or("");
    tracing::info!("check_update [{}]: imagen={}", name, image_full);
    let (has_update, local_tag, remote_digest, remote_tag, error) = if image_full.is_empty() {
        tracing::warn!("check_update [{}]: contenedor sin imagen", name);
        (None, "unknown".into(), None, None, Some("no image".into()))
    } else {
        let local_tag = crate::updates::digest::parse_image_ref(image_full).tag;
        let (remote_digest, remote_tag, error) =
            match check_remote_digest_with_docker(image_full, &docker).await {
                Ok((_manifest, config, tag)) => (Some(config), Some(tag), None),
                Err(e) => {
                    tracing::warn!(
                        "check_update [{}]: error obteniendo digest remoto: {}",
                        name,
                        e
                    );
                    (None, None, Some(e))
                }
            };
        let has_update = match (&container.image_id, &remote_digest) {
            (Some(local_digest), Some(remote_digest)) => {
                let local_short = crate::updates::digest::short_digest(local_digest);
                let remote_short = crate::updates::digest::short_digest(remote_digest);
                let result = local_short != remote_short;
                tracing::info!(
                    "check_update [{}]: local={} remote={} has_update={}",
                    name,
                    local_short,
                    remote_short,
                    result
                );
                Some(result)
            }
            _ => None,
        };
        (has_update, local_tag, remote_digest, remote_tag, error)
    };
    // Persist has_update to database
    if let Some(hu) = has_update {
        let conn = db_pool.get().await.map_err(AppError::from_pool)?;
        let guard = conn.lock().map_err(AppError::from_pool)?;
        let _ = db::update_container_has_update(&guard, &name, hu);
        drop(guard);
        drop(conn);
    }
    let local_digest = container
        .image_id
        .as_ref()
        .map(|d| crate::updates::digest::short_digest(d));
    Ok(Json(VersionCompare {
        local_tag,
        remote_tag,
        has_update,
        local_digest,
        remote_digest: remote_digest.map(|d| crate::updates::digest::short_digest(&d)),
        changelog_url: None,
        error,
    }))
}

/// Check all containers sequentially, applying policies inline.
/// Returns the container list with has_update flags updated.
/// This replaces the old parallel check + background apply pattern to avoid
/// Docker Hub rate limiting (429 errors).
#[allow(clippy::too_many_arguments)]
async fn check_and_apply_all(
    docker: &Docker,
    db_pool: &DbPool,
    tx: &broadcast::Sender<StateEvent>,
    state_notify: &Arc<Notify>,
    progress_cache: &Arc<Mutex<BatchProgress>>,
    progress_tx: &broadcast::Sender<UpdateProgress>,
    cancel_check: &Arc<AtomicBool>,
    settings: &Arc<Mutex<Settings>>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
    terminal_failed: &TerminalFailed,
    rolled_back: &RolledBackGuard,
) -> Vec<ContainerInfo> {
    // Best-effort: si la BD no está disponible se registra y el batch arranca
    // con lista vacía (no hay respuesta HTTP que degradar aquí).
    let mut containers = fetch_containers(docker, &None, db_pool)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("check_and_apply_all: fetch_containers falló: {e}");
            Vec::new()
        });
    let raw_containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
    tracing::info!(
        "check_and_apply_all: verificando {} contenedores",
        containers.len()
    );

    let check_interval = {
        let s = settings.lock().await;
        s.effective_check_interval_ms()
    };

    // Pre-build a name→(&ContainerSummary, image_id) map from raw containers
    let raw_map: HashMap<String, (&bollard::models::ContainerSummary, String)> = raw_containers
        .iter()
        .filter_map(|ct| {
            let name = ct
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| crate::models::strip_name(n))?;
            let image_id = ct.image_id.as_deref().unwrap_or("").to_string();
            Some((name, (ct, image_id)))
        })
        .collect();

    // Load last_remote_digest from DB for accurate comparison (best-effort: con
    // mapa vacío la comparación cae al `image_id`, pero el fallo se registra).
    let last_remote_digest_map: HashMap<String, String> = match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => crate::db::load_last_remote_digest_map(&guard),
            Err(e) => {
                tracing::warn!(
                    "check_and_apply_all: mutex de DB envenenado leyendo last_remote_digest: {e}"
                );
                HashMap::new()
            }
        },
        Err(e) => {
            tracing::warn!(
                "check_and_apply_all: pool de DB no disponible leyendo last_remote_digest: {e}"
            );
            HashMap::new()
        }
    };

    // Load last_pulled_digest from DB (local image cache) to skip redundant pulls
    // (best-effort: sin él se reintenta el pull, pero el fallo se registra).
    let last_pulled_digest_map: HashMap<String, String> = match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => crate::db::load_last_pulled_digest_map(&guard),
            Err(e) => {
                tracing::warn!(
                    "check_and_apply_all: mutex de DB envenenado leyendo last_pulled_digest: {e}"
                );
                HashMap::new()
            }
        },
        Err(e) => {
            tracing::warn!(
                "check_and_apply_all: pool de DB no disponible leyendo last_pulled_digest: {e}"
            );
            HashMap::new()
        }
    };

    let mut any_success = false;
    let total = containers.len() as u32;
    let mut checked = 0u32;
    let mut updated = 0u32;
    let mut updatable = 0u32;
    let mut uptodate = 0u32;
    let mut failed = 0u32;
    let mut num_errors = 0u32;

    for c in &mut containers {
        let name = c.name.clone();
        let image_full = if c.image_tag.is_empty() {
            c.image.clone()
        } else {
            format!("{}:{}", c.image, c.image_tag)
        };

        if image_full.is_empty() {
            tracing::warn!("check_and_apply_all [{}]: sin imagen, omitiendo", name);
            continue;
        }

        // Get image_id and cid from raw map
        let (raw, image_id) = match raw_map.get(name.as_str()) {
            Some((ct, id)) => (*ct, id.clone()),
            None => {
                tracing::warn!(
                    "check_and_apply_all [{}]: no encontrado en raw_containers, omitiendo",
                    name
                );
                continue;
            }
        };

        tracing::info!(
            "check_and_apply_all [{}]: verificando imagen {}",
            name,
            image_full
        );

        checked += 1;

        // Send progress event so frontend shows live feedback
        update_progress(
            progress_cache,
            state_notify,
            progress_tx,
            total,
            checked,
            updated,
            num_errors,
            name.clone(),
        )
        .await;

        match check_remote_digest_with_docker(&image_full, docker).await {
            Ok((manifest_digest, config_digest, _)) => {
                // Use last_remote_digest from DB if available, fallback to image_id.
                // This correctly handles non-DockerHub registries where image_id
                // becomes a manifest digest after recreate with image@manifest_digest.
                let local_ref = last_remote_digest_map
                    .get(&name)
                    .map(|s| s.as_str())
                    .unwrap_or(&image_id);
                let has_update = crate::updates::common::needs_update(local_ref, &config_digest);

                tracing::info!(
                    "check_and_apply_all [{}]: local={} remote={} has_update={}",
                    name,
                    crate::updates::digest::short_digest(&image_id),
                    crate::updates::digest::short_digest(&config_digest),
                    has_update,
                );

                // Update has_update in container and DB (best-effort: el fallo se
                // registra, pero la comprobación del resto continúa).
                c.has_update = has_update;
                match db_pool.get().await {
                    Ok(conn) => match conn.lock() {
                        Ok(conn_lock) => {
                            if let Err(e) =
                                db::update_container_has_update(&conn_lock, &name, has_update)
                            {
                                tracing::error!(
                                    "check_and_apply_all: error updating has_update for '{}': {}",
                                    name,
                                    e
                                );
                            }
                            // `last_remote_digest` represents the image the container is
                            // actually running, so it MUST NOT advance merely because a
                            // newer remote digest was observed. Seed it once (when there
                            // was no stored value and no update pending) so subsequent
                            // cycles have a baseline; the recreate paths persist it after
                            // a successful recreate.
                            if should_seed_last_remote_digest(
                                last_remote_digest_map.contains_key(&name),
                                has_update,
                            ) {
                                if let Err(e) = db::update_container_last_remote_digest(
                                    &conn_lock,
                                    &name,
                                    &config_digest,
                                ) {
                                    tracing::error!(
                                        "check_and_apply_all: error seeding last_remote_digest for '{}': {}",
                                        name,
                                        e
                                    );
                                }
                            }
                            drop(conn_lock);
                        }
                        Err(e) => tracing::warn!(
                            "check_and_apply_all [{}]: mutex de DB envenenado persistiendo has_update: {e}",
                            name
                        ),
                    },
                    Err(e) => tracing::warn!(
                        "check_and_apply_all [{}]: pool de DB no disponible persistiendo has_update: {e}",
                        name
                    ),
                }

                // If has_update and running → apply policy inline
                if has_update {
                    updatable += 1;
                    if c.state == "running" {
                        let cid = raw.id.as_deref().unwrap_or("").to_string();
                        let compose_project = raw
                            .labels
                            .as_ref()
                            .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                            .cloned();
                        let pending = PendingUpdate {
                            name: name.clone(),
                            image_full: image_full.clone(),
                            cid,
                            image_id,
                            remote_digest: Some(config_digest),
                            manifest_digest: Some(manifest_digest),
                            last_pulled_digest: last_pulled_digest_map.get(&name).cloned(),
                            compose_project,
                        };
                        // Resolve policy for this single container
                        let policies = update_policies.lock().await.clone();
                        let policy_map: HashMap<String, UpdatePolicy> = policies
                            .into_iter()
                            .map(|p| (p.container.clone(), p))
                            .collect();
                        // Sin política específica se construye el fallback desde
                        // los defaults de `Settings`.
                        let policy = match policy_map.get(&name) {
                            Some(p) => p.clone(),
                            None => settings.lock().await.fallback_update_policy(name.clone()),
                        };
                        let mut this_ok = false;
                        let outcome = apply_single_policy(
                            docker,
                            settings,
                            state_notify,
                            progress_cache,
                            progress_tx,
                            update_history,
                            db_pool,
                            tx,
                            terminal_failed,
                            rolled_back,
                            &pending,
                            &policy,
                            &mut this_ok,
                            total,
                            checked,
                            updated,
                            num_errors,
                        )
                        .await;
                        any_success |= this_ok;
                        tally_apply_outcome(outcome, &mut updated, &mut failed);
                    }
                } else {
                    // No update needed — mark check as done
                    uptodate += 1;
                    update_progress(
                        progress_cache,
                        state_notify,
                        progress_tx,
                        total,
                        checked,
                        updated,
                        num_errors,
                        name.clone(),
                    )
                    .await;
                }
            }
            Err(e) => {
                num_errors += 1;
                if e.contains("429") {
                    tracing::warn!(
                        "check_and_apply_all [{}]: rate limit (429) fetching digest, saltando",
                        name
                    );
                } else if e.contains("404") {
                    tracing::warn!(
                        "check_and_apply_all [{}]: imagen remota no encontrada (404): {}",
                        name,
                        image_full
                    );
                } else if e.contains("403") {
                    tracing::warn!(
                        "check_and_apply_all [{}]: acceso denegado (403) a: {}",
                        name,
                        image_full
                    );
                } else {
                    tracing::warn!(
                        "check_and_apply_all [{}]: error consultando digest remoto: {}",
                        name,
                        e
                    );
                }
                update_progress(
                    progress_cache,
                    state_notify,
                    progress_tx,
                    total,
                    checked,
                    updated,
                    num_errors,
                    name.clone(),
                )
                .await;
            }
        }

        // Sleep between checks to avoid hammering registries
        if check_interval > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(check_interval)).await;
        }

        // Check if user cancelled
        if cancel_check.load(Ordering::SeqCst) {
            tracing::info!(
                "check_and_apply_all: cancelado por el usuario tras procesar {} contenedores",
                checked
            );
            // Send cancellation progress event
            update_progress(
                progress_cache,
                state_notify,
                progress_tx,
                total,
                checked,
                updated,
                num_errors,
                String::new(),
            )
            .await;
            break;
        }
    }

    // Update timestamps. Compute everything that needs `settings` first so the
    // (non-`Send`) `conn.lock()` guard is never held across an `.await`.
    let (last_check, next_check, settings_snapshot) = {
        let mut s = settings.lock().await;
        let cron = s.effective_update_check_cron();
        let last_check = crate::timezone::now_formatted();
        let next_check = crate::timezone::next_cron_time(&cron).unwrap_or_default();
        s.update_check_last_run_at = Some(last_check.clone());
        (last_check, next_check, s.clone())
    };

    // Best-effort: persiste timestamps y settings; si la BD falla la respuesta
    // (lista de contenedores) ya está calculada y solo se registra el fallo.
    match db_pool.get().await {
        Ok(conn) => match conn.lock() {
            Ok(guard) => {
                for c in &containers {
                    if let Err(e) =
                        db::update_container_check_times(&guard, &c.name, &last_check, &next_check)
                    {
                        tracing::warn!(
                            "check_and_apply_all: no se pudo persistir check_times de '{}': {e}",
                            c.name
                        );
                    }
                }
                if let Err(e) = db::save_settings(&guard, &settings_snapshot) {
                    tracing::warn!("check_and_apply_all: no se pudo persistir settings: {e}");
                }
            }
            Err(e) => tracing::warn!(
                "check_and_apply_all: mutex de DB envenenado persistiendo timestamps: {e}"
            ),
        },
        Err(e) => tracing::warn!(
            "check_and_apply_all: pool de DB no disponible persistiendo timestamps: {e}"
        ),
    }

    // Send StateEvent if any success
    if any_success {
        tracing::info!("check_and_apply_all: enviando StateEvent para refrescar frontend");
        let containers = fetch_containers(docker, &None, db_pool)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("check_and_apply_all: fetch_containers (refresh) falló: {e}");
                Vec::new()
            });
        let _ = tx.send(StateEvent { containers });
    }

    tracing::info!(
        "[check_and_apply_all] FINAL: checked={} total={} updated={} errors={} any_success={}",
        checked,
        total,
        updated,
        num_errors,
        any_success
    );

    // Emit the end-of-batch event with the REAL counters (done = true).
    emit_batch_done(
        progress_cache,
        state_notify,
        progress_tx,
        total,
        checked,
        updated,
        num_errors,
        updatable,
        uptodate,
        failed,
    )
    .await;

    // Reset progress cache after batch completes so the next state poll
    // returns default (idle) progress rather than stale batch data.
    reset_progress_cache(progress_cache, state_notify, progress_tx).await;

    containers
}

#[allow(clippy::too_many_arguments)]
pub async fn check_all_h(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(tx): State<broadcast::Sender<StateEvent>>,
    State(progress_cache): State<Arc<Mutex<BatchProgress>>>,
    State(progress_tx): State<broadcast::Sender<UpdateProgress>>,
    State(cancel_check): State<Arc<AtomicBool>>,
    State(settings): State<Arc<Mutex<Settings>>>,
    State(update_history): State<Arc<Mutex<Vec<UpdateHistoryEntry>>>>,
    State(update_policies): State<Arc<Mutex<Vec<UpdatePolicy>>>>,
    State(state_notify): State<Arc<Notify>>,
    State(terminal_failed): State<TerminalFailed>,
    State(rolled_back): State<RolledBackGuard>,
) -> Json<Vec<ContainerInfo>> {
    // Reset cancel flag at start
    cancel_check.store(false, Ordering::SeqCst);
    let containers = check_and_apply_all(
        &docker,
        &db_pool,
        &tx,
        &state_notify,
        &progress_cache,
        &progress_tx,
        &cancel_check,
        &settings,
        &update_history,
        &update_policies,
        &terminal_failed,
        &rolled_back,
    )
    .await;
    Json(containers)
}

/// Cancel a running batch check/update operation.
pub async fn cancel_check_all_h(State(cancel_check): State<Arc<AtomicBool>>) -> Json<&'static str> {
    cancel_check.store(true, Ordering::SeqCst);
    tracing::info!("cancel_check_all_h: batch cancelado por el usuario");
    Json("cancelled")
}

/// Desenlace de aplicar la política a un contenedor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyOutcome {
    /// La actualización se aplicó con éxito.
    Applied,
    /// No procedía aplicar (política `none` o pull ya en caché): no es fallo.
    Skipped,
    /// Se intentó aplicar y falló.
    Failed,
    /// La guarda revirtió a la imagen anterior (`rolled-back`): es un fallo
    /// aplicado (la actualización no se completó), nunca un skip.
    RolledBack,
    /// La guarda acabó en fallo terminal (`failed-stopped`): fallo aplicado.
    FailedStopped,
}

/// Apply a single container's update policy inline.
/// Contains the same logic as the per-container loop body in `check_and_apply_all`.
#[allow(clippy::too_many_arguments)]
async fn apply_single_policy(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    state_notify: &Arc<Notify>,
    progress_cache: &Arc<Mutex<BatchProgress>>,
    progress_tx: &broadcast::Sender<UpdateProgress>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    db_pool: &DbPool,
    _tx: &broadcast::Sender<StateEvent>,
    terminal_failed: &TerminalFailed,
    rolled_back: &RolledBackGuard,
    p: &PendingUpdate,
    policy: &UpdatePolicy,
    any_success: &mut bool,
    total: u32,
    checked: u32,
    updated: u32,
    errors: u32,
) -> ApplyOutcome {
    if policy.action == UpdateAction::None {
        tracing::warn!(
            "apply_single_policy: política None para '{}', saltando",
            p.name
        );
        let _ = update_progress(
            progress_cache,
            state_notify,
            progress_tx,
            total,
            checked,
            updated,
            errors,
            p.name.clone(),
        )
        .await;
        return ApplyOutcome::Skipped;
    }

    tracing::info!(
        "apply_single_policy: procesando '{}' con acción {:?} (imagen: {}, cleanup_old_image: {})",
        p.name,
        policy.action,
        p.image_full,
        policy.cleanup_old_image
    );

    let _ = update_progress(
        progress_cache,
        state_notify,
        progress_tx,
        total,
        checked,
        updated,
        errors,
        p.name.clone(),
    )
    .await;

    let start_time = std::time::Instant::now();
    let mut success = false;
    let (pull_timeout, guard_max, guard_interval, guard_window) = {
        let s = settings.lock().await;
        let (max, interval, window) = guard_params(policy, &s);
        (s.effective_pull_timeout_secs(), max, interval, window)
    };
    // Desenlace de la guarda, solo cuando `rollback_on_failure` la ejecuta.
    let mut guard_outcome: Option<GuardOutcome> = None;
    let mut guard_attempts: u32 = 0;

    match policy.action {
        UpdateAction::Pull => {
            tracing::info!(
                "apply_single_policy: Pull '{}' desde '{}'",
                p.name,
                p.image_full
            );
            if should_skip_pull(
                &policy.action,
                p.remote_digest.as_deref(),
                p.last_pulled_digest.as_deref(),
            ) {
                tracing::info!(
                    "apply_single_policy: Pull '{}' omitido, imagen ya en caché ({})",
                    p.name,
                    p.remote_digest.as_deref().unwrap_or("")
                );
                update_progress(
                    progress_cache,
                    state_notify,
                    progress_tx,
                    total,
                    checked,
                    updated,
                    errors,
                    p.name.clone(),
                )
                .await;
                // D6: el skip es un no-op real. Salir antes del bloque
                // `if success` para no notificar, ni registrar historial,
                // ni ejecutar prune. El desenlace es `Skipped`: no es
                // ni éxito ni fallo.
                return ApplyOutcome::Skipped;
            } else if pull_image(
                docker,
                &p.image_full,
                p.manifest_digest.as_deref(),
                pull_timeout,
            )
            .await
            {
                tracing::info!("apply_single_policy: Pull OK '{}'", p.name);
                persist_last_pulled_digest(db_pool, &p.name, p.remote_digest.as_deref()).await;
                update_progress(
                    progress_cache,
                    state_notify,
                    progress_tx,
                    total,
                    checked,
                    updated,
                    errors,
                    p.name.clone(),
                )
                .await;
                success = true;
            } else {
                tracing::error!("apply_single_policy: Pull FALLÓ '{}'", p.name);
                let entry = UpdateHistoryEntry {
                    container: p.name.clone(),
                    image: p.image_full.clone(),
                    old_digest: p.image_id.clone(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "apply-policy-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    attempts: 0,
                };
                // Best-effort: persistir el historial no altera el resultado.
                push_history_entry(update_history, db_pool, entry).await;
            }
        }
        UpdateAction::PullRestart => {
            tracing::info!(
                "apply_single_policy: PullRestart '{}' desde '{}'",
                p.name,
                p.image_full
            );
            if pull_image(
                docker,
                &p.image_full,
                p.manifest_digest.as_deref(),
                pull_timeout,
            )
            .await
            {
                tracing::info!(
                    "apply_single_policy: Pull OK, reiniciando contenedor '{}' (cid: {})",
                    p.name,
                    p.cid
                );
                update_progress(
                    progress_cache,
                    state_notify,
                    progress_tx,
                    total,
                    checked,
                    updated,
                    errors,
                    p.name.clone(),
                )
                .await;
                match apply_with_guard(
                    docker,
                    &p.name,
                    &p.cid,
                    &p.image_full,
                    p.manifest_digest.as_deref(),
                    &p.image_id,
                    guard_max,
                    guard_interval,
                    guard_window,
                    policy.rollback_on_failure,
                )
                .await
                {
                    Ok(guard) => {
                        tracing::info!(
                            "apply_single_policy: contenedor '{}' recreado (intentos: {})",
                            p.name,
                            guard.attempts
                        );
                        guard_attempts = guard.attempts;
                        if may_remove_old_image(guard.outcome) {
                            success = true;
                        } else {
                            guard_outcome = Some(guard.outcome);
                        }
                        update_progress(
                            progress_cache,
                            state_notify,
                            progress_tx,
                            total,
                            checked,
                            updated,
                            errors,
                            p.name.clone(),
                        )
                        .await;
                    }
                    Err(e) => {
                        tracing::error!(
                            "apply_single_policy: error al reiniciar '{}': {}",
                            p.name,
                            e
                        );
                        update_progress(
                            progress_cache,
                            state_notify,
                            progress_tx,
                            total,
                            checked,
                            updated,
                            errors,
                            p.name.clone(),
                        )
                        .await;
                        push_history_entry(
                            update_history,
                            db_pool,
                            UpdateHistoryEntry {
                                container: p.name.clone(),
                                image: p.image_full.clone(),
                                old_digest: p.image_id.clone(),
                                new_digest: String::new(),
                                timestamp: crate::timezone::now_formatted(),
                                status: "apply-policy-restart-error".into(),
                                duration_ms: start_time.elapsed().as_millis() as u64,
                                attempts: 0,
                            },
                        )
                        .await;
                    }
                }
            } else {
                tracing::error!("apply_single_policy: Pull FALLÓ '{}'", p.name);
                let entry = UpdateHistoryEntry {
                    container: p.name.clone(),
                    image: p.image_full.clone(),
                    old_digest: p.image_id.clone(),
                    new_digest: String::new(),
                    timestamp: crate::timezone::now_formatted(),
                    status: "apply-policy-error".into(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    attempts: 0,
                };
                // Best-effort: persistir el historial no altera el resultado.
                push_history_entry(update_history, db_pool, entry).await;
            }
        }
        UpdateAction::PullRestartStack => {
            // API-only path (no `docker compose` CLI): pull + recreate the
            // service container via Bollard, same semantics as the scheduler.
            match p.compose_project {
                Some(ref project) => tracing::info!(
                    "apply_single_policy: PullRestartStack '{}' (proyecto: {})",
                    p.name,
                    project
                ),
                None => tracing::info!(
                    "apply_single_policy: PullRestartStack '{}' (sin proyecto)",
                    p.name
                ),
            }
            // La misma guarda que en `PullRestart` (reintentos → rollback por
            // digest) se aplica a los servicios de stack, a través del helper
            // compartido (D11): pull + recrear + guarda.
            let applied = if pull_image(
                docker,
                &p.image_full,
                p.manifest_digest.as_deref(),
                pull_timeout,
            )
            .await
            {
                apply_with_guard(
                    docker,
                    &p.name,
                    &p.cid,
                    &p.image_full,
                    p.manifest_digest.as_deref(),
                    &p.image_id,
                    guard_max,
                    guard_interval,
                    guard_window,
                    policy.rollback_on_failure,
                )
                .await
            } else {
                Err(AppError::Docker(format!(
                    "pull de la imagen '{}' falló",
                    p.image_full
                )))
            };

            match applied {
                Ok(guard) => {
                    tracing::info!(
                        "apply_single_policy: PullRestartStack '{}' recreado vía API (intentos: {})",
                        p.name,
                        guard.attempts
                    );
                    guard_attempts = guard.attempts;
                    if may_remove_old_image(guard.outcome) {
                        success = true;
                    } else {
                        guard_outcome = Some(guard.outcome);
                    }
                    update_progress(
                        progress_cache,
                        state_notify,
                        progress_tx,
                        total,
                        checked,
                        updated,
                        errors,
                        p.name.clone(),
                    )
                    .await;
                }
                Err(e) => {
                    tracing::error!(
                        "apply_single_policy: PullRestartStack '{}' falló: {}",
                        p.name,
                        e
                    );
                    update_progress(
                        progress_cache,
                        state_notify,
                        progress_tx,
                        total,
                        checked,
                        updated,
                        errors,
                        p.name.clone(),
                    )
                    .await;
                }
            }
        }
        _ => {
            update_progress(
                progress_cache,
                state_notify,
                progress_tx,
                total,
                checked,
                updated,
                errors,
                p.name.clone(),
            )
            .await;
        }
    }

    if success {
        tracing::info!("apply_single_policy: éxito '{}'", p.name);
        notify_all(settings, &p.name, "✅ actualizado y reiniciado").await;

        // Recreate actions also performed a successful pull, so record the
        // resulting cache digest (the `Pull` branch persists it inline).
        if should_advance_digest(&policy.action) {
            persist_last_pulled_digest(db_pool, &p.name, p.remote_digest.as_deref()).await;
        }

        // Digest of the image now in use (for history and, when the container
        // was recreated, for persisting `last_remote_digest`).
        // For stacks the recreate is API-driven, so prefer the locally
        // inspected config digest of the redeployed service.
        let new_digest = if policy.action == UpdateAction::PullRestartStack {
            let local = resolve_container_image_digest(docker, &p.name).await;
            if local.is_empty() {
                p.remote_digest.clone().unwrap_or_default()
            } else {
                local
            }
        } else if let Some(d) = &p.remote_digest {
            d.clone()
        } else {
            check_remote_digest_on_image(&p.image_full, docker).await
        };

        // `last_remote_digest` represents the image the container is actually
        // running, so it advances ONLY when the container was recreated.
        // A Pull-only policy updates the local image cache but leaves the
        // running container untouched → it MUST NOT clear `has_update` nor
        // advance the digest.
        let recreated = should_advance_digest(&policy.action);
        if let Ok(conn) = db_pool.get().await {
            if let Ok(guard) = conn.lock() {
                if recreated {
                    if let Err(e) = db::update_container_has_update(&guard, &p.name, false) {
                        tracing::error!(
                            "apply_single_policy: error clearing has_update for '{}': {}",
                            p.name,
                            e
                        );
                    }
                }
                if recreated && !new_digest.is_empty() {
                    if let Err(e) =
                        db::update_container_last_remote_digest(&guard, &p.name, &new_digest)
                    {
                        tracing::error!(
                            "apply_single_policy: error storing last_remote_digest for '{}': {}",
                            p.name,
                            e
                        );
                    }
                } else if recreated {
                    tracing::warn!("apply_single_policy: no se pudo obtener digest remoto para '{}', last_remote_digest no actualizado", p.name);
                }
            } else {
                tracing::error!(
                    "apply_single_policy: mutex de DB poisoned para '{}'",
                    p.name
                );
            }
        } else {
            tracing::error!(
                "apply_single_policy: no se pudo obtener conexión DB para '{}'",
                p.name
            );
        }
        let entry = UpdateHistoryEntry {
            container: p.name.clone(),
            image: p.image_full.clone(),
            old_digest: p.image_id.clone(),
            new_digest: new_digest.clone(),
            timestamp: crate::timezone::now_formatted(),
            status: "success".into(),
            duration_ms: start_time.elapsed().as_millis() as u64,
            attempts: guard_attempts,
        };
        // Best-effort: el historial ya está en memoria; si la BD falla solo se
        // registra (push_history_entry lo hace) y el resultado no cambia.
        push_history_entry(update_history, db_pool, entry).await;

        *any_success = true;

        // Limpiar imágenes dangling si la política lo indica
        if policy.cleanup_old_image {
            tracing::info!(
                "apply_single_policy: policy.cleanup_old_image=true, haciendo prune de imágenes dangling para '{}'",
                p.name
            );
            let result = prune_dangling_images(docker).await;
            log_prune_result("policy.cleanup_old_image", &result);
        }
        ApplyOutcome::Applied
    } else if let Some(outcome) = guard_outcome {
        // La guarda se ejecutó y acabó en rollback (`rolled-back`) o fallo
        // terminal (`failed-stopped`): historial, notificación con el nivel
        // acorde (warn/error) y efectos en memoria (anti-churn D10 / terminal
        // D9) vía el helper compartido. La imagen anterior se conserva.
        record_guard_outcome(
            outcome,
            GuardOutcomeReport {
                name: &p.name,
                image: &p.image_full,
                old_digest: &p.image_id,
                attempted_digest: p.remote_digest.as_deref().unwrap_or(""),
                attempts: guard_attempts,
                duration_ms: start_time.elapsed().as_millis() as u64,
                message: guard_outcome_message(outcome),
            },
            update_history,
            db_pool,
            settings,
            terminal_failed,
            rolled_back,
            true,
        )
        .await;
        match outcome {
            GuardOutcome::RolledBack => ApplyOutcome::RolledBack,
            GuardOutcome::FailedStopped => ApplyOutcome::FailedStopped,
            // No alcanzable: `guard_outcome` solo se fija para no-`Updated`.
            GuardOutcome::Updated => ApplyOutcome::Applied,
        }
    } else {
        tracing::warn!("apply_single_policy: fallo/no-hubo-éxito '{}'", p.name);
        ApplyOutcome::Failed
    }
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
async fn apply_policies_background(
    docker: &Docker,
    settings: &Arc<Mutex<Settings>>,
    state_notify: &Arc<Notify>,
    tx: &broadcast::Sender<StateEvent>,
    progress_cache: &Arc<Mutex<BatchProgress>>,
    progress_tx: &broadcast::Sender<UpdateProgress>,
    update_history: &Arc<Mutex<Vec<UpdateHistoryEntry>>>,
    update_policies: &Arc<Mutex<Vec<UpdatePolicy>>>,
    db_pool: &DbPool,
    pending: &[PendingUpdate],
    terminal_failed: &TerminalFailed,
    rolled_back: &RolledBackGuard,
) {
    tracing::info!(
        "apply_policies_background: iniciando con {} pendientes: {:?}",
        pending.len(),
        pending.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
    let policies = update_policies.lock().await.clone();
    let policy_map: HashMap<String, UpdatePolicy> = policies
        .into_iter()
        .map(|p| (p.container.clone(), p))
        .collect();
    let mut any_success = false;

    for p in pending {
        // Sin política específica se construye el fallback desde los defaults
        // de `Settings`.
        let policy = match policy_map.get(&p.name) {
            Some(pol) => pol.clone(),
            None => settings.lock().await.fallback_update_policy(p.name.clone()),
        };
        let _ = apply_single_policy(
            docker,
            settings,
            state_notify,
            progress_cache,
            progress_tx,
            update_history,
            db_pool,
            tx,
            terminal_failed,
            rolled_back,
            p,
            &policy,
            &mut any_success,
            0, // total
            0, // checked
            0, // updated
            0, // errors
        )
        .await;
    }

    // Safety net: limpiar dangling images que hayan podido quedar
    tracing::info!("apply_policies_background: completado, iniciando prune de imágenes dangling");
    let result = prune_dangling_images(docker).await;
    log_prune_result("safety-net", &result);

    // Enviar StateEvent para que el frontend refresque inmediatamente
    if any_success {
        tracing::info!("apply_policies_background: enviando StateEvent para refrescar frontend");
        let containers = fetch_containers(docker, &None, db_pool)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("apply_policies_background: fetch_containers (refresh) falló: {e}");
                Vec::new()
            });
        let _ = tx.send(StateEvent { containers });
    }
}

/// Prune dangling images (no tag, no container reference)
pub(crate) async fn prune_dangling_images(
    docker: &Docker,
) -> Result<ImagePruneResponse, bollard::errors::Error> {
    let mut filters = HashMap::new();
    filters.insert("dangling", vec!["true"]);
    let opts = PruneImagesOptions { filters };
    docker.prune_images(Some(opts)).await
}

/// Log the result of a prune operation
pub(crate) fn log_prune_result(
    context: &str,
    result: &Result<ImagePruneResponse, bollard::errors::Error>,
) {
    match result {
        Ok(resp) => {
            let deleted_count = resp.images_deleted.as_ref().map(|v| v.len()).unwrap_or(0);
            let reclaimed = resp.space_reclaimed.unwrap_or(0);
            if deleted_count > 0 {
                tracing::info!(
                    "prune_images [{}]: {} imágenes eliminadas, {} bytes liberados",
                    context,
                    deleted_count,
                    reclaimed
                );
            } else {
                tracing::info!(
                    "prune_images [{}]: no hay imágenes dangling para eliminar",
                    context
                );
            }
        }
        Err(e) => {
            tracing::warn!("prune_images [{}]: error: {}", context, e);
        }
    }
}

/// Llama a `check_remote_digest_with_docker` con la referencia de imagen completa.
/// Retorna el digest remoto o cadena vacía si falla la consulta.
async fn check_remote_digest_on_image(image_full: &str, docker: &Docker) -> String {
    match crate::updates::digest::check_remote_digest_with_docker(image_full, docker).await {
        Ok((_manifest, config, _)) => config,
        Err(_) => String::new(),
    }
}

/// Inspect a container and return the ID of the image it is running (the
/// local config digest). Returns an empty string if the container cannot be
/// inspected.
async fn resolve_container_image_digest(docker: &Docker, name: &str) -> String {
    match docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
    {
        Ok(resp) => resp.image.unwrap_or_default(),
        Err(e) => {
            tracing::warn!(
                "resolve_container_image_digest [{}]: inspect failed: {}",
                name,
                e
            );
            String::new()
        }
    }
}

// ── Test helpers (implemented in GREEN phase) ──────────────

/// Emite el evento de fin de batch (`done = true`) con los contadores reales.
#[allow(clippy::too_many_arguments)]
async fn emit_batch_done(
    progress_cache: &Arc<Mutex<BatchProgress>>,
    state_notify: &Arc<Notify>,
    progress_tx: &broadcast::Sender<UpdateProgress>,
    total: u32,
    checked: u32,
    updated: u32,
    errors: u32,
    updatable: u32,
    uptodate: u32,
    failed: u32,
) {
    let mut cache = progress_cache.lock().await;
    cache.total = total;
    cache.checked = checked;
    cache.updated = updated;
    cache.errors = errors;
    cache.updatable = updatable;
    cache.uptodate = uptodate;
    cache.failed = failed;
    cache.checking = String::new();
    drop(cache);
    state_notify.notify_waiters();
    let _ = progress_tx.send(UpdateProgress {
        container: String::new(),
        status: "done".into(),
        done: true,
        error: None,
        total,
        checked,
        updated,
        errors,
        updatable,
        uptodate,
        failed,
    });
}

/// Contabiliza el desenlace de UN solo contenedor. `Skipped` no cuenta.
/// Un rollback sano (`RolledBack`) o un fallo terminal (`FailedStopped`) son
/// fallos aplicados: cuentan como `failed`, nunca como skip.
fn tally_apply_outcome(outcome: ApplyOutcome, updated: &mut u32, failed: &mut u32) {
    match outcome {
        ApplyOutcome::Applied => *updated += 1,
        ApplyOutcome::Skipped => {}
        ApplyOutcome::Failed | ApplyOutcome::RolledBack | ApplyOutcome::FailedStopped => {
            *failed += 1;
        }
    }
}

/// Reset the progress cache to default and notify waiting state handlers.
/// This MUST be called after `check_and_apply_all` completes to clear
/// the "checking" field and prevent stale batch progress on the frontend.
pub(crate) async fn reset_progress_cache(
    progress_cache: &Arc<Mutex<BatchProgress>>,
    state_notify: &Arc<Notify>,
    progress_tx: &broadcast::Sender<UpdateProgress>,
) {
    let mut cache = progress_cache.lock().await;
    *cache = BatchProgress::default();
    drop(cache);
    state_notify.notify_waiters();
    // Do NOT signal end-of-batch here: that is `emit_batch_done`'s job.
    let _ = progress_tx.send(UpdateProgress {
        container: String::new(),
        status: "idle".into(),
        done: false,
        error: None,
        total: 0,
        checked: 0,
        updated: 0,
        errors: 0,
        ..Default::default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{Mutex, Notify};

    /// Verifies that `reset_progress_cache` resets a populated progress cache to default.
    ///
    /// **Bug**: `check_and_apply_all` sends a final `update_progress` with total/checked/updated/errors
    /// values but never resets the `progress_cache` to `BatchProgress::default()`. This leaves stale
    /// batch progress on the frontend until the next batch cycle.
    ///
    /// **Expected behavior**: After `check_and_apply_all` completes, `reset_progress_cache` is called
    /// to zero out all fields and clear the "checking" marker.
    #[tokio::test]
    async fn test_reset_progress_cache_clears_non_default_values() {
        let progress_cache: Arc<Mutex<BatchProgress>> = Arc::new(Mutex::new(BatchProgress {
            total: 5,
            checked: 5,
            updated: 2,
            errors: 0,
            checking: "nginx".into(),
            ..Default::default()
        }));
        let state_notify = Arc::new(Notify::new());
        let (progress_tx, _rx) = broadcast::channel::<UpdateProgress>(8);

        reset_progress_cache(&progress_cache, &state_notify, &progress_tx).await;

        let cache = progress_cache.lock().await;
        assert_eq!(cache.total, 0, "total should be reset to 0");
        assert_eq!(cache.checked, 0, "checked should be reset to 0");
        assert_eq!(cache.updated, 0, "updated should be reset to 0");
        assert_eq!(cache.errors, 0, "errors should be reset to 0");
        assert!(cache.checking.is_empty(), "checking should be cleared");
    }

    // ── recreate_image_ref ───────────────────────────────────

    /// `recreate_container` MUST write a clean `repo:tag` image reference to
    /// `Config.Image`, never a `repo:tag@sha256:...` pin generated by Alloy.
    #[test]
    fn test_recreate_image_ref_strips_digest_pin() {
        assert_eq!(
            recreate_image_ref("gitea/gitea:1.27.3@sha256:87a6deadbeef"),
            "gitea/gitea:1.27.3"
        );
    }

    #[test]
    fn test_recreate_image_ref_is_idempotent() {
        assert_eq!(recreate_image_ref("nginx:alpine"), "nginx:alpine");
        assert_eq!(recreate_image_ref("nginx"), "nginx");
    }

    // ── should_seed_last_remote_digest ───────────────────────

    /// Seed only when there is no stored digest and no update pending.
    #[test]
    fn test_should_seed_only_when_missing_and_no_update() {
        assert!(should_seed_last_remote_digest(false, false));
        assert!(!should_seed_last_remote_digest(false, true));
        assert!(!should_seed_last_remote_digest(true, false));
        assert!(!should_seed_last_remote_digest(true, true));
    }

    // ── should_advance_digest (gating de `recreated`) ────────

    /// Only recreate actions advance `last_remote_digest` (and clear
    /// `has_update`); pull-only and none leave the running image untouched.
    #[test]
    fn test_should_advance_digest_only_on_recreate() {
        assert!(should_advance_digest(&UpdateAction::PullRestart));
        assert!(should_advance_digest(&UpdateAction::PullRestartStack));
        assert!(!should_advance_digest(&UpdateAction::Pull));
        assert!(!should_advance_digest(&UpdateAction::None));
    }

    // ── should_skip_pull (pull-cache-digest) ─────────────────

    /// Scenario: Imagen ya en caché — Pull + digests iguales → skip.
    #[test]
    fn test_should_skip_pull_when_remote_matches_cache() {
        assert!(should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
    }

    /// Scenario: Imagen nueva en el remoto — Pull + digests distintos → pull.
    #[test]
    fn test_should_skip_pull_false_when_remote_differs() {
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            Some("sha256:aaa")
        ));
    }

    /// Only `Pull` may skip; recreate actions always pull.
    #[test]
    fn test_should_skip_pull_only_for_pull_action() {
        assert!(!should_skip_pull(
            &UpdateAction::PullRestart,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::PullRestartStack,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::None,
            Some("sha256:bbb"),
            Some("sha256:bbb")
        ));
    }

    /// Both digests must be present.
    #[test]
    fn test_should_skip_pull_requires_both_digests() {
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            None,
            Some("sha256:bbb")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            None
        ));
        assert!(!should_skip_pull(&UpdateAction::Pull, None, None));
    }

    /// Empty digests must never match.
    #[test]
    fn test_should_skip_pull_false_on_empty_digests() {
        assert!(!should_skip_pull(&UpdateAction::Pull, Some(""), Some("")));
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some("sha256:bbb"),
            Some("")
        ));
        assert!(!should_skip_pull(
            &UpdateAction::Pull,
            Some(""),
            Some("sha256:bbb")
        ));
    }

    /// Scenario: El skip es un no-op (D6). Un `Pull` cuyo digest remoto ya
    /// coincide con el cacheado NO debe marcar éxito: sin notificación, sin
    /// entrada de historial y sin prune (aunque `cleanup_old_image=true`).
    /// Solo se registra log y se actualiza el progreso.
    #[tokio::test]
    async fn test_pull_skip_is_noop() {
        // HTTP transport never touches the local socket, so the test runs
        // without a Docker daemon; the skip path makes no Docker calls.
        let docker = Docker::connect_with_http_defaults().expect("docker client");
        let settings = Arc::new(Mutex::new(Settings::default()));
        let state_notify = Arc::new(Notify::new());
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let update_history: Arc<Mutex<Vec<UpdateHistoryEntry>>> = Arc::new(Mutex::new(Vec::new()));
        let db_pool = db::test_pool();
        let (tx, _rx) = broadcast::channel::<StateEvent>(8);
        let (progress_tx, _progress_rx) = broadcast::channel::<UpdateProgress>(8);

        let p = PendingUpdate {
            name: "web".into(),
            image_full: "nginx:latest".into(),
            cid: "cid123".into(),
            image_id: "sha256:old".into(),
            remote_digest: Some("sha256:bbb".into()),
            manifest_digest: None,
            last_pulled_digest: Some("sha256:bbb".into()),
            compose_project: None,
        };
        let policy = UpdatePolicy {
            container: "web".into(),
            action: UpdateAction::Pull,
            cleanup_old_image: true,
            rollback_on_failure: false,
            notify_events: true,
            max_start_attempts: None,
            health_window_secs: None,
        };
        let mut any_success = false;
        let terminal_failed = TerminalFailed::default();
        let rolled_back = RolledBackGuard::default();

        let outcome = apply_single_policy(
            &docker,
            &settings,
            &state_notify,
            &progress_cache,
            &progress_tx,
            &update_history,
            &db_pool,
            &tx,
            &terminal_failed,
            &rolled_back,
            &p,
            &policy,
            &mut any_success,
            1,
            1,
            0,
            0,
        )
        .await;

        assert_eq!(
            outcome,
            super::ApplyOutcome::Skipped,
            "un skip de pull no es ni éxito ni fallo"
        );
        assert!(!any_success, "el skip no debe marcar `any_success`");
        assert!(
            update_history.lock().await.is_empty(),
            "el skip no debe añadir entrada al historial"
        );
    }

    // ── progress emission on the SSE channel (sse-first, task 1.4) ──

    /// `update_progress` MUST emit an `update-progress` event carrying the real
    /// counters, in addition to waking the long-poll handler.
    #[tokio::test]
    async fn test_update_progress_emits_update_progress_event() {
        let progress_cache: Arc<Mutex<BatchProgress>> =
            Arc::new(Mutex::new(BatchProgress::default()));
        let state_notify = Arc::new(Notify::new());
        let (progress_tx, mut rx) = broadcast::channel::<UpdateProgress>(8);

        update_progress(
            &progress_cache,
            &state_notify,
            &progress_tx,
            5,
            3,
            1,
            0,
            "nginx".into(),
        )
        .await;

        let evt = rx
            .try_recv()
            .expect("update-progress event must be emitted");
        assert_eq!(evt.total, 5);
        assert_eq!(evt.checked, 3);
        assert_eq!(evt.updated, 1);
        assert_eq!(evt.errors, 0);
        assert_eq!(evt.container, "nginx");
        assert!(!evt.done, "in-flight progress is not the end of a batch");
    }

    /// `reset_progress_cache` NO debe señalar fin de batch con ceros; el fin
    /// lo emite `emit_batch_done` con contadores reales.
    #[tokio::test]
    async fn test_reset_progress_cache_does_not_signal_done_with_zeros() {
        let progress_cache: Arc<Mutex<BatchProgress>> = Arc::new(Mutex::new(BatchProgress {
            total: 5,
            checked: 5,
            updated: 2,
            errors: 0,
            checking: "nginx".into(),
            updatable: 1,
            uptodate: 4,
            failed: 0,
        }));
        let state_notify = Arc::new(Notify::new());
        let (progress_tx, mut rx) = broadcast::channel::<UpdateProgress>(8);

        reset_progress_cache(&progress_cache, &state_notify, &progress_tx).await;

        let evt = rx
            .try_recv()
            .expect("reset debe emitir un evento update-progress");
        assert!(
            !evt.done,
            "reset no debe emitir done=true (lo hace emit_batch_done)"
        );
        assert_eq!(evt.total, 0);
        assert_eq!(evt.checked, 0);
    }

    /// El evento de fin de batch debe llevar los contadores reales y done=true.
    #[tokio::test]
    async fn test_emit_batch_done_carries_real_counters() {
        let progress_cache = Arc::new(Mutex::new(BatchProgress::default()));
        let state_notify = Arc::new(Notify::new());
        let (progress_tx, mut rx) = broadcast::channel::<UpdateProgress>(8);

        emit_batch_done(
            &progress_cache,
            &state_notify,
            &progress_tx,
            10,
            10,
            2,
            1,
            3,
            7,
            1,
        )
        .await;

        let evt = rx.try_recv().expect("done event must be emitted");
        assert!(evt.done, "el evento de fin debe llevar done=true");
        assert_eq!(evt.total, 10);
        assert_eq!(evt.checked, 10);
        assert_eq!(evt.updated, 2);
        assert_eq!(evt.errors, 1);
        assert_eq!(evt.updatable, 3);
        assert_eq!(evt.uptodate, 7);
        assert_eq!(evt.failed, 1);
    }

    /// El recuento distingue aplicado / omitido / fallido por contenedor.
    #[test]
    fn test_tally_apply_outcome_counts_each_apply_independently() {
        use super::ApplyOutcome;
        let mut updated = 0u32;
        let mut failed = 0u32;
        for outcome in [
            ApplyOutcome::Applied,
            ApplyOutcome::Applied,
            ApplyOutcome::Skipped,
            ApplyOutcome::Failed,
            ApplyOutcome::Applied,
        ] {
            tally_apply_outcome(outcome, &mut updated, &mut failed);
        }
        assert_eq!(updated, 3, "3 aplicaciones correctas");
        assert_eq!(failed, 1, "1 aplicación fallida (el skip no cuenta)");
    }

    /// Un rollback sano o un fallo terminal son fallos aplicados, nunca un skip.
    #[test]
    fn test_tally_apply_outcome_guard_failures_count_as_failed() {
        let mut updated = 0u32;
        let mut failed = 0u32;
        for outcome in [
            ApplyOutcome::RolledBack,
            ApplyOutcome::FailedStopped,
            ApplyOutcome::Skipped,
        ] {
            tally_apply_outcome(outcome, &mut updated, &mut failed);
        }
        assert_eq!(updated, 0, "ningún rollback cuenta como actualizado");
        assert_eq!(failed, 2, "rollback y fallo terminal son fallos aplicados");
    }

    // ── guard_params (override de política gana) ─────────────

    fn guard_test_policy(
        max_start_attempts: Option<u32>,
        health_window_secs: Option<u64>,
    ) -> UpdatePolicy {
        UpdatePolicy {
            container: "web".into(),
            action: UpdateAction::PullRestart,
            cleanup_old_image: false,
            rollback_on_failure: true,
            notify_events: false,
            max_start_attempts,
            health_window_secs,
        }
    }

    #[test]
    fn test_guard_params_policy_override_wins() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            ..Default::default()
        };
        let policy = guard_test_policy(Some(5), Some(60));
        assert_eq!(guard_params(&policy, &settings), (5, 10, 60));
    }

    #[test]
    fn test_guard_params_uses_globals_without_override() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            ..Default::default()
        };
        let policy = guard_test_policy(None, None);
        assert_eq!(guard_params(&policy, &settings), (3, 10, 30));
    }

    // ── guard_params × overrides por defecto (precedencia) ───

    /// Un contenedor SIN política usa el override de la política por defecto
    /// (sembrado en su `UpdatePolicy` de fallback), no el global.
    #[test]
    fn test_guard_params_default_override_applies_to_policy_less_container() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            default_max_start_attempts: Some(5),
            ..Default::default()
        };
        // La `UpdatePolicy` de fallback se siembra con los overrides por defecto.
        let (def_max, def_window) = settings.default_guard_overrides();
        let policy = guard_test_policy(def_max, def_window);
        assert_eq!(guard_params(&policy, &settings), (5, 10, 30));
    }

    /// Escenario «Limpiar» del fallback: sin override por defecto (`None`) la
    /// política de fallback cae al global (`update_max_start_attempts`).
    #[test]
    fn test_guard_params_fallback_without_default_override_uses_global() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            default_max_start_attempts: None,
            ..Default::default()
        };
        let policy = settings.fallback_update_policy("web");
        assert_eq!(guard_params(&policy, &settings), (3, 10, 30));
    }

    /// Cableado extremo a extremo: el fallback construido por el helper lleva
    /// los overrides por defecto y `guard_params` los resuelve (sin pasar por
    /// `guard_test_policy`).
    #[test]
    fn test_fallback_update_policy_integrates_with_guard_params() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            default_max_start_attempts: Some(5),
            default_health_window_secs: Some(60),
            ..Default::default()
        };
        let policy = settings.fallback_update_policy("web");
        assert_eq!(guard_params(&policy, &settings), (5, 10, 60));
    }

    /// La política del contenedor gana sobre el override por defecto.
    #[test]
    fn test_guard_params_container_policy_override_beats_default() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            default_max_start_attempts: Some(5),
            ..Default::default()
        };
        let policy = guard_test_policy(Some(7), None);
        assert_eq!(guard_params(&policy, &settings), (7, 10, 30));
    }

    /// Un contenedor CON política sin override cae al global, no al default
    /// (el override por defecto solo aplica al fallback sin política).
    #[test]
    fn test_guard_params_container_policy_without_override_uses_global() {
        let settings = Settings {
            update_max_start_attempts: 3,
            update_start_attempt_interval_secs: 10,
            update_health_window_secs: 30,
            default_max_start_attempts: Some(5),
            ..Default::default()
        };
        let policy = guard_test_policy(None, None);
        assert_eq!(guard_params(&policy, &settings), (3, 10, 30));
    }

    /// `max_start_attempts = 0` se normaliza a 1 para no provocar bucles infinitos.
    #[test]
    fn test_guard_params_clamps_zero_max_attempts() {
        let settings = Settings::default();
        let policy = guard_test_policy(Some(0), None);
        assert_eq!(guard_params(&policy, &settings).0, 1);
    }

    // ── clear_suppression (D9/D10, intervención manual) ──────

    /// El helper compartido por los flujos manuales MUST limpiar ambos guards:
    /// el estado terminal `failed-stopped` (`terminal_failed`) y el anti-churn de
    /// rollback (`rolled_back`).
    #[tokio::test]
    async fn clear_suppression_clears_both_guards() {
        let terminal_failed = TerminalFailed::default();
        let rolled_back = RolledBackGuard::default();
        terminal_failed.0.lock().await.insert("app".into());
        rolled_back
            .0
            .lock()
            .await
            .insert("app".into(), "sha256:deadbeef".into());

        clear_suppression("app", &terminal_failed, &rolled_back).await;

        assert!(
            !terminal_failed.0.lock().await.contains("app"),
            "terminal_failed debe quedar limpio tras la intervención manual"
        );
        assert!(
            !rolled_back.0.lock().await.contains_key("app"),
            "rolled_back debe quedar limpio tras la intervención manual"
        );
    }

    /// La limpieza MUST estar acotada al contenedor intervenido: no debe tocar la
    /// supresión de otros contenedores.
    #[tokio::test]
    async fn clear_suppression_is_scoped_per_container() {
        let terminal_failed = TerminalFailed::default();
        let rolled_back = RolledBackGuard::default();
        terminal_failed.0.lock().await.insert("app".into());
        terminal_failed.0.lock().await.insert("db".into());
        rolled_back
            .0
            .lock()
            .await
            .insert("app".into(), "sha256:new".into());
        rolled_back
            .0
            .lock()
            .await
            .insert("db".into(), "sha256:new".into());

        clear_suppression("app", &terminal_failed, &rolled_back).await;

        let tf = terminal_failed.0.lock().await;
        let rb = rolled_back.0.lock().await;
        assert!(tf.contains("db"), "db sigue suprimido (no intervenido)");
        assert!(rb.contains_key("db"), "db sigue suprimido (no intervenido)");
        assert!(!tf.contains("app"));
        assert!(!rb.contains_key("app"));
    }

    /// Limpiar una supresión inexistente es un no-op (idempotente): los flujos
    /// masivos llaman al helper para cada contenedor iterado, haya supresión o no.
    #[tokio::test]
    async fn clear_suppression_is_noop_when_absent() {
        let terminal_failed = TerminalFailed::default();
        let rolled_back = RolledBackGuard::default();

        clear_suppression("ghost", &terminal_failed, &rolled_back).await;

        assert!(terminal_failed.0.lock().await.is_empty());
        assert!(rolled_back.0.lock().await.is_empty());
    }

    /// Contrato de bucle de `update_all_h` / `update_stack_h`: cada contenedor
    /// iterado pasa por `clear_suppression` antes de aplicar, así que una tanda
    /// completa deja la supresión de todos ellos limpia.
    ///
    /// Los handlers completos requieren un daemon de contenedores
    /// (`docker_list_running` / `docker.list_containers`), que el repo sólo
    /// ejercita en tests `#[ignore]` con `podman_client`; este test cubre el
    /// contrato de limpieza por contenedor sin daemon.
    #[tokio::test]
    async fn clear_suppression_clears_every_iterated_container() {
        let terminal_failed = TerminalFailed::default();
        let rolled_back = RolledBackGuard::default();
        let iterated = ["app", "db", "cache"];
        for name in iterated {
            terminal_failed.0.lock().await.insert(name.to_string());
            rolled_back
                .0
                .lock()
                .await
                .insert(name.to_string(), "sha256:remote".into());
        }

        // Espejo del bucle de los handlers: limpiar por contenedor iterado.
        for name in iterated {
            clear_suppression(name, &terminal_failed, &rolled_back).await;
        }

        assert!(
            terminal_failed.0.lock().await.is_empty(),
            "ningún contenedor iterado debe quedar en terminal_failed"
        );
        assert!(
            rolled_back.0.lock().await.is_empty(),
            "ningún contenedor iterado debe quedar suprimido por rollback"
        );
    }
}
