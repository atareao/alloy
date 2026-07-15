use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{delete, get, post},
    Router,
};
use bollard::{
    image::{ListImagesOptions, PruneImagesOptions, RemoveImageOptions},
    Docker,
};

use crate::models::ImageInfo;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct ListImagesQuery {
    pub show_all: Option<bool>,
}

/// Filtra imágenes `<none>:<none>` (dangling) a menos que `show_all=true`.
fn is_dangling(repo_tags: &[String]) -> bool {
    repo_tags.is_empty()
        || repo_tags
            .iter()
            .all(|t| t == "<none>:<none>" || t.starts_with("<none>:"))
}

fn image_repo_tag(i: &bollard::models::ImageSummary) -> (String, String, String) {
    let repo_tag = i
        .repo_tags
        .first()
        .filter(|t| !t.starts_with("<none>:"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "<none>:<none>".into());

    let (repo, tag) = if let Some(pos) = repo_tag.rfind(':') {
        (repo_tag[..pos].to_string(), repo_tag[pos + 1..].to_string())
    } else {
        (repo_tag.clone(), "latest".into())
    };
    (repo, tag, repo_tag)
}

async fn list_images_h(
    State(docker): State<Docker>,
    Query(query): Query<ListImagesQuery>,
) -> Json<Vec<ImageInfo>> {
    let images = docker
        .list_images(Some(ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();

    let show_all = query.show_all.unwrap_or(false);

    let result: Vec<ImageInfo> = images
        .iter()
        .filter(|i| show_all || !is_dangling(&i.repo_tags))
        .map(|i| {
            let (repo, tag, _) = image_repo_tag(i);

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

async fn remove_image_h(
    State(docker): State<Docker>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match docker
        .remove_image(
            &id,
            Some(RemoveImageOptions {
                force: false,
                noprune: false,
            }),
            None,
        )
        .await
    {
        Ok(_) => Json(serde_json::json!({"status": "deleted", "id": id})),
        Err(e) => Json(serde_json::json!({"status": "error", "id": id, "error": e.to_string()})),
    }
}

async fn prune_images_h(State(docker): State<Docker>) -> Json<serde_json::Value> {
    match docker
        .prune_images(Some(PruneImagesOptions::<&str> {
            ..Default::default()
        }))
        .await
    {
        Ok(report) => {
            let value: serde_json::Value = serde_json::to_value(&report).unwrap_or_default();
            let deleted = value
                .get("ImagesDeleted")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            Json(serde_json::json!({"status": "pruned", "images_deleted": deleted}))
        }
        Err(e) => Json(serde_json::json!({"status": "error", "error": e.to_string()})),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/images", get(list_images_h))
        .route("/api/images/{id}", delete(remove_image_h))
        .route("/api/images/prune", post(prune_images_h))
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
        let result: Json<Vec<ImageInfo>> = list_images_h(
            State(docker),
            Query(ListImagesQuery {
                show_all: Some(true),
            }),
        )
        .await;
        assert!(!result.0.is_empty(), "Should have at least one image");
    }

    #[tokio::test]
    async fn test_integration_list_images_filters_dangling() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<ImageInfo>> = list_images_h(
            State(docker),
            Query(ListImagesQuery {
                show_all: Some(false),
            }),
        )
        .await;
        for img in &result.0 {
            assert!(
                !img.repo.starts_with("<none>"),
                "Dangling image should be filtered: {}",
                img.repo
            );
        }
    }

    #[tokio::test]
    async fn test_integration_list_images_structure() {
        if !is_podman_available() {
            eprintln!("SKIP: Podman not available");
            return;
        }
        let docker = podman_client().await;
        let result: Json<Vec<ImageInfo>> = list_images_h(
            State(docker),
            Query(ListImagesQuery {
                show_all: Some(true),
            }),
        )
        .await;
        for img in &result.0 {
            assert!(!img.id.is_empty(), "Image ID should not be empty");
            assert!(img.id.len() <= 12, "ID truncated to 12 chars: {}", img.id);
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
        let result: Json<Vec<ImageInfo>> = list_images_h(
            State(docker),
            Query(ListImagesQuery {
                show_all: Some(true),
            }),
        )
        .await;
        let alloy = result.0.iter().find(|img| img.repo.contains("alloy"));
        assert!(alloy.is_some(), "Image 'alloy' should be present");
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
