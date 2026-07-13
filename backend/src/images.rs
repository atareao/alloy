use axum::{extract::State, response::Json, routing::get, Router};
use bollard::{image::ListImagesOptions, Docker};

use crate::models::ImageInfo;
use crate::state::AppState;

#[allow(clippy::let_and_return)]
async fn list_images_h(State(docker): State<Docker>) -> Json<Vec<ImageInfo>> {
    let images = docker
        .list_images(Some(ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();

    let result: Vec<ImageInfo> = images
        .iter()
        .map(|i| {
            let repo_tag = i
                .repo_tags
                .first()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "<none>:<none>".into());

            let (repo, tag) = if let Some(pos) = repo_tag.rfind(':') {
                (repo_tag[..pos].to_string(), repo_tag[pos + 1..].to_string())
            } else {
                (repo_tag.clone(), "latest".into())
            };

            ImageInfo {
                id: i
                    .id
                    .split(':')
                    .next_back()
                    .unwrap_or("")
                    .chars()
                    .take(12)
                    .collect(),
                repo,
                tag,
                size_mb: ((i.size as f64) / 1_048_576.0 * 100.0).round() / 100.0,
                virtual_size_mb: i.virtual_size.map_or(0.0, |vs| {
                    ((vs as f64) / 1_048_576.0 * 100.0).round() / 100.0
                }),
                created: i.created,
                containers: i.containers,
                repo_tags: i.repo_tags.clone(),
                repo_digests: i.repo_digests.clone(),
                labels: i.labels.clone(),
            }
        })
        .collect();

    Json(result)
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/images", get(list_images_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_podman_available() -> bool {
        std::env::var("DOCKER_HOST").is_ok()
            || std::path::Path::new("/run/user/1000/podman/podman.sock").exists()
    }

    async fn podman_client() -> Docker {
        let socket = std::env::var("DOCKER_HOST")
            .unwrap_or_else(|_| "unix:///run/user/1000/podman/podman.sock".to_string());
        Docker::connect_with_local(&socket, 120, bollard::API_DEFAULT_VERSION)
            .expect("Failed to connect to Podman socket")
    }

    #[tokio::test]
    async fn test_integration_list_images_returns_list() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<ImageInfo>> = list_images_h(State(docker)).await;
        assert!(!result.0.is_empty(), "Should have at least one image");
    }

    #[tokio::test]
    async fn test_integration_list_images_structure() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<ImageInfo>> = list_images_h(State(docker)).await;
        for img in &result.0 {
            assert!(!img.id.is_empty(), "Image ID should not be empty");
            assert!(img.id.len() <= 12, "ID truncated to 12 chars: {}", img.id);
            assert!(!img.repo.is_empty(), "Repo should not be empty");
            assert!(!img.tag.is_empty(), "Tag should not be empty");
            assert!(img.size_mb >= 0.0, "Size >= 0 for {}", img.repo);
            assert!(
                img.virtual_size_mb >= 0.0,
                "Virtual size >= 0 for {}",
                img.repo
            );
            assert!(img.created > 0, "Created > 0 for {}", img.repo);
        }
    }

    #[tokio::test]
    async fn test_integration_list_images_alloy_present() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<ImageInfo>> = list_images_h(State(docker)).await;
        let alloy = result.0.iter().find(|img| img.repo.contains("alloy"));
        assert!(alloy.is_some(), "Image 'alloy' should be present");
    }

    #[tokio::test]
    async fn test_integration_list_images_repo_tags_nonempty() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<ImageInfo>> = list_images_h(State(docker)).await;
        for img in &result.0 {
            if img.repo != "<none>" {
                assert!(
                    !img.repo_tags.is_empty(),
                    "repo_tags empty for {}",
                    img.repo
                );
            }
        }
    }

    #[test]
    fn test_size_conversion_100mb() {
        let bytes: i64 = 104_857_600;
        let mb = ((bytes as f64) / 1_048_576.0 * 100.0).round() / 100.0;
        assert_eq!(mb, 100.0);
    }

    #[test]
    fn test_size_conversion_1mb() {
        let bytes: i64 = 1_048_576;
        let mb = ((bytes as f64) / 1_048_576.0 * 100.0).round() / 100.0;
        assert_eq!(mb, 1.0);
    }

    #[test]
    fn test_size_conversion_zero() {
        let bytes: i64 = 0;
        let mb = ((bytes as f64) / 1_048_576.0 * 100.0).round() / 100.0;
        assert_eq!(mb, 0.0);
    }
}
