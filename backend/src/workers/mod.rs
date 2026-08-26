pub mod scheduler;
pub mod state;

pub use scheduler::resolve_compose_file;
pub use scheduler::update_check_worker;
pub use state::docker_list_running;
pub use state::state_worker;
pub use state::CachedContainers;

/// Worker de limpieza: pruna imágenes dangling cada 6 horas.
pub async fn cleanup_worker(docker: Docker) {
    let mut tick = tokio::time::interval(Duration::from_secs(6 * 3600));
    tick.tick().await; // skip first
    loop {
        tick.tick().await;
        tracing::info!("🧹 cleanup_worker: prunando imágenes dangling");
        match docker
            .prune_images(None::<bollard::image::PruneImagesOptions<&str>>)
            .await
        {
            Ok(report) => tracing::info!(
                "🧹 cleanup_worker: prune completado, {} bytes liberados",
                report.space_reclaimed.unwrap_or(0)
            ),
            Err(e) => tracing::warn!("🧹 cleanup_worker: error en prune: {}", e),
        }
    }
}

use bollard::Docker;
use std::time::Duration;
