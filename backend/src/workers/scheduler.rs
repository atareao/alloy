use bollard::{container::ListContainersOptions, Docker};
use chrono::Local;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::db::DbPool;
use crate::models::*;
use crate::updates::digest::check_remote_digest;

/// Worker que ejecuta revisiones de actualizaciones según el cron configurado.
/// Solo revisa las imágenes para marcar si tienen actualización pendiente.
/// NO hace pull ni restart — eso lo gestionan las políticas desde `apply_policies_background`.
pub async fn update_check_worker(docker: Docker, settings: Arc<Mutex<Settings>>, db_pool: DbPool) {
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
        let now = Local::now();
        let expr = format!("0 {}", cron);
        let should_run = match expr.parse::<cron::Schedule>() {
            Ok(schedule) => schedule.includes(now.to_utc()),
            Err(e) => {
                tracing::warn!("update_check: cron inválido '{}': {}", cron, e);
                false
            }
        };
        if !should_run {
            continue;
        }

        tracing::info!("update_check: revisando actualizaciones (cron={})", cron);

        // 1. Obtener todos los contenedores
        let containers = docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap_or_default();

        // 2. Revisar cada contenedor en paralelo
        let tasks: Vec<_> = containers
            .iter()
            .map(|c| {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| crate::models::strip_name(n))
                    .unwrap_or_default();
                let image_full = c.image.as_deref().unwrap_or("").to_string();
                let image_id = c.image_id.clone().unwrap_or_default();
                async move {
                    if image_full.is_empty() {
                        return (name, false);
                    }
                    let (repo, local_tag) = crate::updates::digest::parse_image_ref(&image_full);
                    match check_remote_digest(&repo, &local_tag).await {
                        Ok((remote_digest, _)) => {
                            let has_update = if !image_id.is_empty() {
                                let local_short = crate::updates::digest::short_digest(&image_id);
                                let remote_short =
                                    crate::updates::digest::short_digest(&remote_digest);
                                local_short != remote_short
                            } else {
                                false
                            };
                            (name, has_update)
                        }
                        Err(_) => (name, false),
                    }
                }
            })
            .collect();

        // 3. Persistir has_update en DB
        {
            let obj = db_pool.get().await.unwrap();
            let mut updated_count = 0u32;
            for (name, has_update) in futures::future::join_all(tasks).await {
                let _ = db::update_container_has_update(&obj.lock().unwrap(), &name, has_update);
                if has_update {
                    updated_count += 1;
                }
            }
            drop(obj);
            tracing::info!(
                "update_check: {} contenedores revisados, {} con actualizaciones",
                containers.len(),
                updated_count
            );
        }
    }
}

pub async fn resolve_compose_file(docker: &Docker, project: &str) -> Option<String> {
    let containers = docker
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();
    let project_containers: Vec<_> = containers
        .iter()
        .filter(|c| {
            c.labels
                .as_ref()
                .and_then(|l| l.get(crate::models::LABEL_COMPOSE_PROJECT))
                .map(|p| p == project)
                .unwrap_or(false)
        })
        .collect();
    if project_containers.is_empty() {
        return None;
    }
    project_containers
        .first()
        .and_then(|c| c.labels.as_ref())
        .and_then(|l| l.get(crate::models::LABEL_COMPOSE_CONFIG_FILES))
        .cloned()
        .or_else(|| {
            project_containers
                .first()
                .and_then(|c| c.labels.as_ref())
                .and_then(|l| l.get(crate::models::LABEL_COMPOSE_WORKING_DIR))
                .map(|dir| format!("{}/docker-compose.yml", dir))
        })
        .filter(|p| std::path::Path::new(p).exists())
}

#[cfg(test)]
mod tests {
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
}
