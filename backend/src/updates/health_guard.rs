//! update-health-guard — núcleo puro de la guarda de actualizaciones.
//!
//! Lógica pura (sin I/O) reutilizada por el worker de updates y por los
//! handlers que aplican políticas: decide cuándo reintentar el arranque,
//! cuándo hacer rollback y qué resultado final registrar. La parte de I/O
//! (inspect/healthcheck/rollback) vive en `containers` y `updates::handlers`.

use std::collections::HashMap;

/// Estado del contenedor que cuenta como "arrancado".
const RUNNING: &str = "running";
/// Resultado de HEALTHCHECK que cuenta como sano.
const HEALTHY: &str = "healthy";
/// Resultado de HEALTHCHECK que invalida el arranque.
const UNHEALTHY: &str = "unhealthy";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHealth {
    pub state: String,
    pub restart_count: u64,
    pub health: Option<String>, // None => the image defines no HEALTHCHECK
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardOutcome {
    Updated,
    RolledBack,
    FailedStopped,
}

/// Build a RuntimeHealth snapshot; `health = None` when no HEALTHCHECK exists.
pub fn runtime_health(state: &str, restart_count: u64, health: Option<&str>) -> RuntimeHealth {
    RuntimeHealth {
        state: state.to_string(),
        restart_count,
        health: health.map(str::to_string),
    }
}

/// True iff the container stayed `running` across all samples, `restart_count` did not
/// increase across samples, and (when a HEALTHCHECK exists) it ends `healthy`.
pub fn is_stable_running(samples: &[RuntimeHealth]) -> bool {
    let Some(first) = samples.first() else {
        return false;
    };

    let stayed_running = samples.iter().all(|s| s.state == RUNNING);
    let no_restart_increase = samples
        .iter()
        .all(|s| s.restart_count <= first.restart_count);

    // Solo se exige HEALTHCHECK si la imagen lo define (alguna muestra lo trae).
    let healthcheck_ok = if samples.iter().any(|s| s.health.is_some()) {
        let never_unhealthy = samples
            .iter()
            .all(|s| s.health.as_deref() != Some(UNHEALTHY));
        let ends_healthy = samples
            .last()
            .and_then(|s| s.health.as_deref())
            .is_some_and(|h| h == HEALTHY);
        never_unhealthy && ends_healthy
    } else {
        true
    };

    stayed_running && no_restart_increase && healthcheck_ok
}

/// True when attempts already made reach/exceed the max => rollback.
/// `max_attempts` is expected to be >= 1; a non-positive max never triggers rollback.
pub fn should_rollback(attempts_made: u32, max_attempts: u32) -> bool {
    max_attempts >= 1 && attempts_made >= max_attempts
}

/// Siguiente paso de la guarda tras un intento de arranque fallido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardStep {
    /// Quedan intentos disponibles: re-arrancar la imagen nueva y re-verificar.
    RetryStart,
    /// Intentos agotados: iniciar el rollback a la imagen anterior.
    RollbackNow,
}

/// Decide el siguiente paso tras un intento de verificación fallido.
///
/// `attempts_made` es el nº de intentos ya realizados (>= 1). Delega el umbral
/// en [`should_rollback`]: con `max_attempts < 1` nunca se hace rollback (el
/// llamador debe normalizar el mínimo a 1).
pub fn next_step(attempts_made: u32, max_attempts: u32) -> GuardStep {
    if should_rollback(attempts_made, max_attempts) {
        GuardStep::RollbackNow
    } else {
        GuardStep::RetryStart
    }
}

/// Resultado final una vez intentado el rollback.
///
/// Si la imagen anterior arranca sana ⇒ [`GuardOutcome::RolledBack`]; si no,
/// el contenedor se deja parado ⇒ [`GuardOutcome::FailedStopped`].
pub fn outcome_after_rollback(old_image_healthy: bool) -> GuardOutcome {
    if old_image_healthy {
        GuardOutcome::RolledBack
    } else {
        GuardOutcome::FailedStopped
    }
}

/// Effective max attempts: policy override wins over global.
pub fn effective_max_attempts(policy_override: Option<u32>, global: u32) -> u32 {
    policy_override.unwrap_or(global)
}

/// Effective health window secs: policy override wins over global.
pub fn effective_health_window(policy_override: Option<u64>, global: u64) -> u64 {
    policy_override.unwrap_or(global)
}

/// Exact digest reference for rollback (must NOT be a `rollback-<ts>` tag).
pub fn rollback_reference(old_digest: &str) -> String {
    old_digest.to_string()
}

/// Notification level: Updated->"info", RolledBack->"warning", FailedStopped->"critical".
pub fn guard_notify_level(outcome: GuardOutcome) -> &'static str {
    match outcome {
        GuardOutcome::Updated => "info",
        GuardOutcome::RolledBack => "warning",
        GuardOutcome::FailedStopped => "critical",
    }
}

/// Estado de historial asociado a un desenlace de la guarda.
pub fn guard_history_status(outcome: GuardOutcome) -> &'static str {
    match outcome {
        GuardOutcome::Updated => "updated",
        GuardOutcome::RolledBack => "rolled-back",
        GuardOutcome::FailedStopped => "failed-stopped",
    }
}

/// La imagen anterior SÓLO puede descartarse tras **confirmar** que la nueva
/// está sana (requisito «Imagen antigua retenida hasta confirmar la nueva
/// sana»). Un rollback (`rolled-back`) o un fallo terminal (`failed-stopped`)
/// la conservan.
pub fn may_remove_old_image(outcome: GuardOutcome) -> bool {
    outcome == GuardOutcome::Updated
}

/// Resultado normalizado de un intento de verificación de arranque sano.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyResult {
    /// El contenedor arrancó sano (ventana estable).
    Healthy,
    /// La verificación concluyó que NO arrancó sano (reintentar o rollback).
    Unhealthy,
    /// Fallo de inspección de Docker: resultado **inconcluso**, nunca un
    /// «no arrancó sano» (design D12).
    InspectionError,
}

impl VerifyResult {
    /// Normaliza un `Result<bool, _>`: `Ok(true)` ⇒ sano, `Ok(false)` ⇒ no
    /// sano, `Err(_)` ⇒ inconcluso. Un error de inspección NUNCA se degrada a
    /// `false` (nada de `unwrap_or(false)`).
    pub fn from_result<E>(result: &Result<bool, E>) -> Self {
        match result {
            Ok(true) => Self::Healthy,
            Ok(false) => Self::Unhealthy,
            Err(_) => Self::InspectionError,
        }
    }
}

/// Acción de la guarda tras un intento de verificación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardAction {
    /// Arranque sano: desenlace `Updated`.
    Accept,
    /// Reintentar el arranque de la imagen nueva.
    RetryStart,
    /// Intentos agotados: rollback a la imagen anterior (teardown).
    Rollback,
    /// Fallo de inspección (inconcluso): ni rollback ni parada, y la imagen
    /// anterior se conserva (design D12).
    SkipTeardown,
}

/// Decide la acción de la guarda tras un intento de verificación.
///
/// Un [`VerifyResult::InspectionError`] NUNCA provoca teardown (rollback o
/// parada) por sí mismo: es un fallo de la API de Docker, no un contenedor que
/// no arranca (design D12). Sólo un [`VerifyResult::Unhealthy`] con los
/// intentos agotados conduce a [`GuardAction::Rollback`].
pub fn decide_after_verify(
    verify: VerifyResult,
    attempts_made: u32,
    max_attempts: u32,
) -> GuardAction {
    match verify {
        VerifyResult::Healthy => GuardAction::Accept,
        VerifyResult::InspectionError => GuardAction::SkipTeardown,
        VerifyResult::Unhealthy => match next_step(attempts_made, max_attempts) {
            GuardStep::RetryStart => GuardAction::RetryStart,
            GuardStep::RollbackNow => GuardAction::Rollback,
        },
    }
}

/// Desenlace tras el rollback a partir del resultado de la verificación de la
/// imagen anterior.
///
/// Un error de inspección es **inconcluso**: el rollback SÍ se aplicó
/// (`rolled-back`) y el contenedor NO se para (design D12), evitando una
/// parada espuria por un fallo transitorio de Docker.
pub fn outcome_after_post_rollback_verify(verify: VerifyResult) -> GuardOutcome {
    match verify {
        VerifyResult::Healthy => outcome_after_rollback(true),
        VerifyResult::Unhealthy => outcome_after_rollback(false),
        VerifyResult::InspectionError => GuardOutcome::RolledBack,
    }
}

/// ¿Debe omitirse el contenedor en este ciclo porque su último update acabó en
/// rollback para el **mismo** digest remoto? (anti-churn, design D10).
///
/// Un digest remoto distinto (nueva versión publicada) NO se suprime: el ciclo
/// MUST reintentar.
pub fn is_rolled_back_suppressed(
    name: &str,
    remote_digest: &str,
    rolled_back: &HashMap<String, String>,
) -> bool {
    rolled_back.get(name).is_some_and(|d| d == remote_digest)
}

/// Registra (inserta/limpia) la supresión anti-churn de un contenedor.
///
/// Simétrico: se inserta `(name, attempted_digest)` tras un `RolledBack` y se
/// elimina ante `has_update = false` o intervención manual del operador
/// (design D10). Es la única escritura del mapa `rolled_back`.
pub fn record_rolled_back(
    rolled_back: &mut HashMap<String, String>,
    name: &str,
    attempted_digest: &str,
    rolled_back_now: bool,
) {
    if rolled_back_now {
        rolled_back.insert(name.to_string(), attempted_digest.to_string());
    } else {
        rolled_back.remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample without HEALTHCHECK (image defines none).
    fn sample(state: &str, restart_count: u64) -> RuntimeHealth {
        RuntimeHealth {
            state: state.to_string(),
            restart_count,
            health: None,
        }
    }

    /// Sample with HEALTHCHECK outcome.
    fn sample_health(state: &str, restart_count: u64, health: &str) -> RuntimeHealth {
        RuntimeHealth {
            state: state.to_string(),
            restart_count,
            health: Some(health.to_string()),
        }
    }

    // --- runtime_health -----------------------------------------------------

    #[test]
    fn runtime_health_builds_snapshot_without_healthcheck() {
        let snapshot = runtime_health("running", 0, None);
        assert_eq!(
            snapshot,
            RuntimeHealth {
                state: "running".to_string(),
                restart_count: 0,
                health: None,
            }
        );
    }

    #[test]
    fn runtime_health_builds_snapshot_with_healthcheck() {
        let snapshot = runtime_health("running", 2, Some("healthy"));
        assert_eq!(
            snapshot,
            RuntimeHealth {
                state: "running".to_string(),
                restart_count: 2,
                health: Some("healthy".to_string()),
            }
        );
    }

    // --- is_stable_running --------------------------------------------------

    #[test]
    fn stable_running_ok_without_healthcheck() {
        let samples = vec![sample("running", 0), sample("running", 0)];
        assert!(is_stable_running(&samples));
    }

    #[test]
    fn stable_running_false_on_restart_count_increase() {
        let samples = vec![sample("running", 0), sample("running", 1)];
        assert!(!is_stable_running(&samples));
    }

    #[test]
    fn stable_running_false_when_not_running() {
        let samples = vec![sample("running", 0), sample("exited", 0)];
        assert!(!is_stable_running(&samples));
    }

    #[test]
    fn stable_running_false_on_unhealthy() {
        let samples = vec![
            sample_health("running", 0, "starting"),
            sample_health("running", 0, "unhealthy"),
        ];
        assert!(!is_stable_running(&samples));
    }

    #[test]
    fn stable_running_true_on_healthy() {
        let samples = vec![
            sample_health("running", 0, "starting"),
            sample_health("running", 0, "healthy"),
        ];
        assert!(is_stable_running(&samples));
    }

    // --- should_rollback ----------------------------------------------------

    #[test]
    fn should_rollback_when_attempts_reach_max() {
        assert!(should_rollback(3, 3));
    }

    #[test]
    fn should_rollback_false_before_max() {
        assert!(!should_rollback(2, 3));
    }

    // --- next_step ----------------------------------------------------------

    #[test]
    fn next_step_retries_before_max() {
        assert_eq!(next_step(1, 3), GuardStep::RetryStart);
        assert_eq!(next_step(2, 3), GuardStep::RetryStart);
    }

    #[test]
    fn next_step_rolls_back_when_attempts_exhausted() {
        assert_eq!(next_step(3, 3), GuardStep::RollbackNow);
        assert_eq!(next_step(4, 3), GuardStep::RollbackNow);
    }

    /// Override por política con `max_start_attempts = 1`: el primer fallo ya
    /// dispara el rollback.
    #[test]
    fn next_step_policy_override_single_attempt_rolls_back() {
        assert_eq!(next_step(1, 1), GuardStep::RollbackNow);
    }

    // --- outcome_after_rollback ---------------------------------------------

    #[test]
    fn outcome_after_rollback_sane_is_rolled_back() {
        assert_eq!(outcome_after_rollback(true), GuardOutcome::RolledBack);
    }

    #[test]
    fn outcome_after_rollback_unhealthy_is_failed_stopped() {
        assert_eq!(outcome_after_rollback(false), GuardOutcome::FailedStopped);
    }

    // --- orquestación simulada (nº de intentos real) ------------------------

    /// Simula el bucle de reintentos usando las funciones puras reales y
    /// devuelve el nº de intentos realizados. `healthy_on` es el intento
    /// (1-based) en el que la imagen nueva arranca sana; `None` = nunca.
    fn simulate_attempts(max_attempts: u32, healthy_on: Option<u32>) -> u32 {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if healthy_on == Some(attempts) {
                return attempts;
            }
            match next_step(attempts, max_attempts) {
                GuardStep::RetryStart => {}
                GuardStep::RollbackNow => return attempts,
            }
        }
    }

    /// Se agotan los N intentos: se realizan exactamente `max_attempts`.
    #[test]
    fn simulated_attempts_equals_max_when_all_fail() {
        assert_eq!(simulate_attempts(3, None), 3);
        assert_eq!(simulate_attempts(5, None), 5);
        assert_eq!(simulate_attempts(1, None), 1);
    }

    /// Arranque sano en un intento posterior: el contador refleja el intento real.
    #[test]
    fn simulated_attempts_reflects_success_attempt() {
        assert_eq!(simulate_attempts(3, Some(1)), 1);
        assert_eq!(simulate_attempts(3, Some(2)), 2);
        assert_eq!(simulate_attempts(3, Some(3)), 3);
    }

    // --- effective_max_attempts / effective_health_window -------------------

    #[test]
    fn effective_max_attempts_uses_global_when_none() {
        assert_eq!(effective_max_attempts(None, 3), 3);
    }

    #[test]
    fn effective_max_attempts_policy_override_wins() {
        assert_eq!(effective_max_attempts(Some(5), 3), 5);
    }

    #[test]
    fn effective_health_window_override_wins() {
        assert_eq!(effective_health_window(Some(60), 30), 60);
    }

    // --- rollback_reference / guard_notify_level ----------------------------

    #[test]
    fn rollback_reference_is_exact_digest_and_not_tag() {
        let reference = rollback_reference("sha256:abc123");
        assert_eq!(reference, "sha256:abc123");
        assert!(!reference.starts_with("rollback-"));
    }

    #[test]
    fn guard_notify_level_mapping() {
        assert_eq!(guard_notify_level(GuardOutcome::Updated), "info");
        assert_eq!(guard_notify_level(GuardOutcome::RolledBack), "warning");
        assert_eq!(guard_notify_level(GuardOutcome::FailedStopped), "critical");
    }

    // --- D12: un error de inspección NUNCA es "no sano" ----------------------

    #[test]
    fn verify_result_normalizes_ok_true_to_healthy() {
        assert_eq!(
            VerifyResult::from_result(&Ok::<bool, String>(true)),
            VerifyResult::Healthy
        );
    }

    #[test]
    fn verify_result_normalizes_ok_false_to_unhealthy() {
        assert_eq!(
            VerifyResult::from_result(&Ok::<bool, String>(false)),
            VerifyResult::Unhealthy
        );
    }

    /// Un `Err` de inspección es **inconcluso**: nunca se degrada a `false`
    /// (nada de `unwrap_or(false)`).
    #[test]
    fn verify_result_normalizes_err_to_inspection_error() {
        assert_eq!(
            VerifyResult::from_result(&Err::<bool, String>("docker down".into())),
            VerifyResult::InspectionError
        );
    }

    // --- D12: decisión de la guarda ante cada intento ------------------------

    #[test]
    fn decide_after_verify_accepts_healthy_start() {
        assert_eq!(
            decide_after_verify(VerifyResult::Healthy, 1, 3),
            GuardAction::Accept
        );
    }

    #[test]
    fn decide_after_verify_retries_unhealthy_before_max() {
        assert_eq!(
            decide_after_verify(VerifyResult::Unhealthy, 1, 3),
            GuardAction::RetryStart
        );
        assert_eq!(
            decide_after_verify(VerifyResult::Unhealthy, 2, 3),
            GuardAction::RetryStart
        );
    }

    #[test]
    fn decide_after_verify_rolls_back_when_attempts_exhausted() {
        assert_eq!(
            decide_after_verify(VerifyResult::Unhealthy, 3, 3),
            GuardAction::Rollback
        );
    }

    /// Un error de inspección (persistente) NO dispara rollback ni parada: se
    /// omite el teardown y la imagen anterior se conserva (design D12).
    #[test]
    fn decide_after_verify_inspection_error_skips_teardown() {
        // Ni con intentos disponibles…
        assert_eq!(
            decide_after_verify(VerifyResult::InspectionError, 1, 3),
            GuardAction::SkipTeardown
        );
        // …ni con los intentos agotados se convierte en un rollback.
        assert_eq!(
            decide_after_verify(VerifyResult::InspectionError, 3, 3),
            GuardAction::SkipTeardown
        );
        assert_ne!(
            decide_after_verify(VerifyResult::InspectionError, 3, 3),
            GuardAction::Rollback
        );
    }

    // --- D12: desenlace tras el rollback ------------------------------------

    #[test]
    fn post_rollback_verify_healthy_is_rolled_back() {
        assert_eq!(
            outcome_after_post_rollback_verify(VerifyResult::Healthy),
            GuardOutcome::RolledBack
        );
    }

    #[test]
    fn post_rollback_verify_unhealthy_is_failed_stopped() {
        assert_eq!(
            outcome_after_post_rollback_verify(VerifyResult::Unhealthy),
            GuardOutcome::FailedStopped
        );
    }

    /// Verificación post-rollback inconclusa: el rollback SÍ se aplicó
    /// (`rolled-back`) y el contenedor NO se para (`failed-stopped`).
    #[test]
    fn post_rollback_verify_inspection_error_does_not_stop() {
        let outcome = outcome_after_post_rollback_verify(VerifyResult::InspectionError);
        assert_eq!(outcome, GuardOutcome::RolledBack);
        assert_ne!(outcome, GuardOutcome::FailedStopped);
    }

    // --- Historial y limpieza de la imagen anterior --------------------------

    #[test]
    fn guard_history_status_mapping() {
        assert_eq!(guard_history_status(GuardOutcome::Updated), "updated");
        assert_eq!(
            guard_history_status(GuardOutcome::RolledBack),
            "rolled-back"
        );
        assert_eq!(
            guard_history_status(GuardOutcome::FailedStopped),
            "failed-stopped"
        );
    }

    /// La imagen anterior sólo se descarta tras confirmar la nueva sana.
    #[test]
    fn may_remove_old_image_only_when_confirmed_healthy() {
        assert!(may_remove_old_image(GuardOutcome::Updated));
        assert!(!may_remove_old_image(GuardOutcome::RolledBack));
        assert!(!may_remove_old_image(GuardOutcome::FailedStopped));
    }

    // --- D10: anti-churn tras `rolled-back` ---------------------------------

    #[test]
    fn rolled_back_suppressed_with_same_digest() {
        let mut map = HashMap::new();
        record_rolled_back(&mut map, "app", "sha256:new", true);
        assert!(is_rolled_back_suppressed("app", "sha256:new", &map));
    }

    /// Un digest remoto distinto (nueva versión publicada) NO se suprime.
    #[test]
    fn rolled_back_not_suppressed_with_different_digest() {
        let mut map = HashMap::new();
        record_rolled_back(&mut map, "app", "sha256:new", true);
        assert!(!is_rolled_back_suppressed("app", "sha256:newer", &map));
    }

    #[test]
    fn rolled_back_not_suppressed_when_absent() {
        let map = HashMap::new();
        assert!(!is_rolled_back_suppressed("app", "sha256:new", &map));
    }

    /// Escrito como `decide`+`record`: el digest resuelto en el ciclo decide la
    /// supresión; registrado un intento revertido, el mismo digest se suprime y
    /// uno nuevo se reintenta.
    #[test]
    fn rolled_back_cycle_suppresses_same_digest_and_retries_new_one() {
        let mut map: HashMap<String, String> = HashMap::new();
        record_rolled_back(&mut map, "app", "sha256:v1", true);
        // Mismo digest remoto ⇒ se salta el contenedor.
        assert!(is_rolled_back_suppressed("app", "sha256:v1", &map));
        // Digest remoto nuevo ⇒ se reintenta.
        assert!(!is_rolled_back_suppressed("app", "sha256:v2", &map));
    }

    #[test]
    fn record_rolled_back_is_symmetric_on_clear() {
        let mut map = HashMap::new();
        record_rolled_back(&mut map, "app", "sha256:new", true);
        assert_eq!(map.get("app").map(|d| d.as_str()), Some("sha256:new"));
        record_rolled_back(&mut map, "app", "", false);
        assert!(
            !map.contains_key("app"),
            "la limpieza (has_update=false / intervención) elimina la entrada"
        );
    }

    #[test]
    fn record_rolled_back_is_idempotent() {
        let mut map = HashMap::new();
        record_rolled_back(&mut map, "app", "sha256:new", true);
        record_rolled_back(&mut map, "app", "sha256:new", true);
        assert_eq!(map.len(), 1, "no debe duplicar entradas");
    }

    #[test]
    fn record_rolled_back_is_scoped_per_container() {
        let mut map = HashMap::new();
        record_rolled_back(&mut map, "a", "sha256:A", true);
        record_rolled_back(&mut map, "b", "sha256:B", true);
        record_rolled_back(&mut map, "a", "", false);
        assert!(!map.contains_key("a"));
        assert_eq!(map.get("b").map(|d| d.as_str()), Some("sha256:B"));
    }
}
