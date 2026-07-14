use crate::state::http_client;

/// Fetch the config digest (image ID) of a remote image from Docker Hub.
///
/// Returns `(config_digest, tag)` where `config_digest` matches what Docker
/// stores locally as `ImageID`, so a byte-for-byte comparison is correct.
///
/// For multi-arch (manifest list) images this performs a second request to
/// resolve the platform-specific manifest and extract its `config.digest`.
pub async fn check_remote_digest(repo: &str, tag: &str) -> Result<(String, String), String> {
    let client = http_client();
    let token_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
        repo
    );
    let token_resp = client
        .get(&token_url)
        .send()
        .await
        .map_err(|e| format!("token request failed: {}", e))?;
    let token_body: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|e| format!("token parse failed: {}", e))?;
    let token = token_body["token"]
        .as_str()
        .ok_or_else(|| "no token".to_string())?;

    let manifest_url = format!("https://registry-1.docker.io/v2/{}/manifests/{}", repo, tag);
    let manifest_resp = client
        .get(&manifest_url)
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.list.v2+json",
        )
        .header("Accept", "application/vnd.oci.image.manifest.v1+json")
        .header("Accept", "application/vnd.oci.image.index.v1+json")
        .send()
        .await
        .map_err(|e| format!("manifest request failed: {}", e))?;
    if !manifest_resp.status().is_success() {
        return Err(format!("manifest status: {}", manifest_resp.status()));
    }

    let content_type = manifest_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let config_digest =
        if content_type.contains("manifest.list") || content_type.contains("image.index") {
            let body: serde_json::Value = manifest_resp
                .json()
                .await
                .map_err(|e| format!("manifest list parse failed: {}", e))?;
            let manifests = body["manifests"]
                .as_array()
                .ok_or_else(|| "no manifests in list".to_string())?;
            let amd64_digest = manifests
                .iter()
                .find(|m| {
                    let plat = &m["platform"];
                    plat["architecture"].as_str() == Some("amd64")
                        && plat["os"].as_str() == Some("linux")
                })
                .or_else(|| manifests.first())
                .and_then(|m| m["digest"].as_str())
                .ok_or_else(|| "no suitable platform manifest".to_string())?;

            let plat_url = format!(
                "https://registry-1.docker.io/v2/{}/manifests/{}",
                repo, amd64_digest
            );
            let plat_resp = client
                .get(&plat_url)
                .header("Authorization", format!("Bearer {}", token))
                .header(
                    "Accept",
                    "application/vnd.docker.distribution.manifest.v2+json",
                )
                .header("Accept", "application/vnd.oci.image.manifest.v1+json")
                .send()
                .await
                .map_err(|e| format!("platform manifest request failed: {}", e))?;
            if !plat_resp.status().is_success() {
                return Err(format!("platform manifest status: {}", plat_resp.status()));
            }
            let plat_body: serde_json::Value = plat_resp
                .json()
                .await
                .map_err(|e| format!("platform manifest parse failed: {}", e))?;
            plat_body["config"]["digest"]
                .as_str()
                .ok_or_else(|| "no config digest in platform manifest".to_string())?
                .to_string()
        } else {
            let body: serde_json::Value = manifest_resp
                .json()
                .await
                .map_err(|e| format!("manifest parse failed: {}", e))?;
            body["config"]["digest"]
                .as_str()
                .ok_or_else(|| "no config digest".to_string())?
                .to_string()
        };

    Ok((config_digest, tag.to_string()))
}

/// Extrae (repo, tag) de un string de imagen.
/// Si no tiene tag, asume "latest". Si tiene digest (@sha256:...), tag = "digest".
pub fn parse_image_ref(image_full: &str) -> (String, String) {
    if let Some(pos) = image_full.rfind('@') {
        (image_full[..pos].to_string(), "digest".to_string())
    } else if let Some(pos) = image_full.rfind(':') {
        (
            image_full[..pos].to_string(),
            image_full[pos + 1..].to_string(),
        )
    } else {
        (image_full.to_string(), "latest".to_string())
    }
}

/// Extrae los primeros 12 caracteres del digest después de ':'.
pub fn short_digest(digest: &str) -> String {
    digest
        .split(':')
        .next_back()
        .unwrap_or("")
        .chars()
        .take(12)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_with_tag() {
        let (repo, tag) = parse_image_ref("nginx:latest");
        assert_eq!(repo, "nginx");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_parse_image_with_version_tag() {
        let (repo, tag) = parse_image_ref("library/postgres:15-alpine");
        assert_eq!(repo, "library/postgres");
        assert_eq!(tag, "15-alpine");
    }

    #[test]
    fn test_parse_image_with_digest() {
        let (repo, tag) = parse_image_ref(
            "nginx@sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert_eq!(repo, "nginx");
        assert_eq!(tag, "digest");
    }

    #[test]
    fn test_parse_image_registry_with_port() {
        let (repo, tag) = parse_image_ref("registry.example.com:5000/myimage:v2");
        assert_eq!(repo, "registry.example.com:5000/myimage");
        assert_eq!(tag, "v2");
    }

    #[test]
    fn test_parse_image_without_tag_defaults_latest() {
        let (repo, tag) = parse_image_ref("alpine");
        assert_eq!(repo, "alpine");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_parse_image_registry_path_with_tag() {
        let (repo, tag) = parse_image_ref("docker.io/library/redis:7.2");
        assert_eq!(repo, "docker.io/library/redis");
        assert_eq!(tag, "7.2");
    }

    #[test]
    fn test_parse_image_registry_path_with_digest() {
        let (repo, tag) = parse_image_ref(
            "docker.io/library/redis@sha256:fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
        );
        assert_eq!(repo, "docker.io/library/redis");
        assert_eq!(tag, "digest");
    }

    #[test]
    fn test_parse_image_empty() {
        let (repo, tag) = parse_image_ref("");
        assert_eq!(repo, "");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn test_short_digest_full() {
        let short =
            short_digest("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
        assert_eq!(short.len(), 12);
        assert_eq!(short, "abc123def456");
    }

    #[test]
    fn test_short_digest_no_colon() {
        let short = short_digest("plainstring");
        assert_eq!(short, "plainstring");
    }

    #[test]
    fn test_short_digest_exactly_12() {
        let short = short_digest("sha256:abcdef123456");
        assert_eq!(short, "abcdef123456");
    }

    #[test]
    fn test_short_digest_less_than_12() {
        let short = short_digest("sha256:abc");
        assert_eq!(short, "abc");
    }

    #[test]
    fn test_short_digest_empty() {
        let short = short_digest("");
        assert_eq!(short, "");
    }

    #[test]
    fn test_short_digest_different() {
        let local =
            short_digest("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
        let remote =
            short_digest("sha256:xyz789ghi012xyz789ghi012xyz789ghi012xyz789ghi012xyz789ghi012xyz7");
        assert_ne!(local, remote);
    }

    #[test]
    fn test_short_digest_same() {
        let d1 =
            short_digest("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
        let d2 =
            short_digest("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1");
        assert_eq!(d1, d2);
    }
}
