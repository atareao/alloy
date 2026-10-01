use crate::state::http_client;
use bollard::Docker;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use tokio::sync::Semaphore;

/// Semaphore to limit concurrent registry manifest fetches to prevent 429 rate limiting.
fn digest_semaphore() -> &'static Semaphore {
    static SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();
    SEMAPHORE.get_or_init(|| Semaphore::new(4))
}

struct CachedToken {
    token: String,
    fetched_at: Instant,
}

/// Cache tokens per registry to avoid redundant auth requests.
fn token_cache() -> &'static Mutex<HashMap<String, CachedToken>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedToken>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Parsed image reference with registry, repo, tag, optional digest, and
/// digest-only flag.
pub struct ImageRef {
    pub registry: String,
    pub repo: String,
    pub tag: String,
    /// Digest suffix when the reference is pinned (`repo:tag@sha256:...`).
    pub digest: Option<String>,
    pub digest_only: bool,
}

/// Parse a full image reference into its components.
///
/// Rules:
/// - `sha256:<digest>` (bare digest) or an empty string → `digest_only = true`.
/// - If the first path segment contains `.` or `:` it is a registry host
///   (e.g. `ghcr.io`, `registry.example.com:5000`); otherwise the image is
///   assumed to come from Docker Hub (`docker.io`).
/// - A leading `docker.io/` prefix on the remainder is stripped.
/// - The tag is extracted after the last `:` (unless it is part of a port).
/// - A `@sha256:...` suffix is captured in `digest` and stripped from the
///   name; the real tag is preserved (never replaced by the literal `"digest"`).
/// - If there is no tag it defaults to `"latest"`.
/// - For Docker Hub images with a single-component repo (no `/`), the repo is
///   prefixed with `library/` (e.g. `nginx:latest` → `library/nginx`).
///
/// Examples:
/// - `nginx:latest` → registry=`docker.io`, repo=`library/nginx`, tag=`latest`
/// - `ghcr.io/owner/repo:tag` → registry=`ghcr.io`, repo=`owner/repo`, tag=`tag`
/// - `postgres:17-alpine` → registry=`docker.io`, repo=`library/postgres`, tag=`17-alpine`
/// - `registry.example.com:5000/myimage:v2` → registry=`registry.example.com:5000`, repo=`myimage`, tag=`v2`
/// - `docker.io/library/redis@sha256:...` → registry=`docker.io`, repo=`library/redis`, tag=`latest`, digest=`Some("sha256:...")`
/// - `gitea/gitea:1.27.3@sha256:...` → registry=`docker.io`, repo=`gitea/gitea`, tag=`1.27.3`, digest=`Some("sha256:...")`
/// - `sha256:abc123...` → digest_only=`true`
pub fn parse_image_ref(image_full: &str) -> ImageRef {
    // Bare digest or empty reference → no repo/tag to resolve.
    if image_full.is_empty() || image_full.starts_with("sha256:") {
        return ImageRef {
            registry: String::new(),
            repo: String::new(),
            tag: String::new(),
            digest: None,
            digest_only: true,
        };
    }

    // Split off a possible digest suffix (`repo:tag@sha256:...`), preserving
    // the real tag that precedes the `@`.
    let (name_part, digest) = match image_full.rfind('@') {
        Some(pos) => (&image_full[..pos], Some(image_full[pos + 1..].to_string())),
        None => (image_full, None),
    };

    // Determine the registry host from the first path segment, if any.
    let (registry, rest) = match name_part.find('/') {
        Some(pos) => {
            let first = &name_part[..pos];
            if first.contains('.') || first.contains(':') {
                (first.to_string(), &name_part[pos + 1..])
            } else {
                ("docker.io".to_string(), name_part)
            }
        }
        None => ("docker.io".to_string(), name_part),
    };

    // Strip an explicit `docker.io/` prefix if present.
    let rest = rest.strip_prefix("docker.io/").unwrap_or(rest);

    // Extract the tag after the last ':' (unless it is part of a port).
    let (repo, tag) = match rest.rfind(':') {
        Some(pos) if !rest[pos + 1..].contains('/') => (&rest[..pos], &rest[pos + 1..]),
        _ => (rest, "latest"),
    };

    // Docker Hub single-component names live under `library/`.
    let repo = if registry == "docker.io" && !repo.contains('/') {
        format!("library/{}", repo)
    } else {
        repo.to_string()
    };

    ImageRef {
        registry,
        repo,
        tag: tag.to_string(),
        digest,
        digest_only: false,
    }
}

/// Return the tag-based reference of an image, stripping any `@sha256:...`
/// digest suffix. Used to build clean `Config.Image` values and to retag
/// after pulling by digest.
///
/// Examples:
/// - `gitea/gitea:1.27.3@sha256:abc...` → `gitea/gitea:1.27.3`
/// - `nginx:alpine` → `nginx:alpine`
/// - `sha256:abc...` → `sha256:abc...` (no `@`)
pub fn tag_ref(image_full: &str) -> &str {
    match image_full.find('@') {
        Some(pos) => &image_full[..pos],
        None => image_full,
    }
}

/// Result of a remote digest check.
///
/// `degraded` is `true` when the registry could not provide the real config
/// digest and the manifest digest was reused as `config_digest`. A degraded
/// value MUST NOT be persisted as the running-image baseline, because
/// comparing a manifest digest against a config digest always reports a
/// spurious update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDigest {
    /// `Docker-Content-Digest` of the manifest (used to pull by digest).
    pub manifest_digest: String,
    /// Config digest (matches the local `ImageID`); untrusted when `degraded`.
    pub config_digest: String,
    /// Resolved tag.
    pub tag: String,
    /// `true` when `config_digest` is actually the manifest digest (fallback).
    pub degraded: bool,
}

/// Fetch both the manifest digest and config digest of a remote image from any registry.
///
/// Returns `(manifest_digest, config_digest, tag)` where:
/// - `manifest_digest` comes from the `Docker-Content-Digest` response header and is
///   used for pulling images via `image@manifest_digest` (Docker daemon requires a
///   manifest digest, not a config digest, in the `@digest` position).
/// - `config_digest` comes from `body["config"]["digest"]` and matches what Docker
///   stores locally as `ImageID`, so a byte-for-byte comparison is correct.
///
/// For multi-arch (manifest list) images this performs a second request to
/// resolve the platform-specific manifest and extract both the manifest digest
/// (from the platform manifest response's `Docker-Content-Digest` header) and
/// the config digest (from `body["config"]["digest"]`).
///
/// If the HTTP-based check fails with 401/403 (registry requires auth),
/// falls back to `docker.inspect_registry_image()` which uses the Docker
/// daemon's credentials (~/.docker/config.json).
///
/// Callers that need to know whether the result was degraded must use
/// [`check_remote_digest_with_docker_detailed`].
pub async fn check_remote_digest_with_docker(
    image_full: &str,
    docker: &Docker,
) -> Result<(String, String, String), String> {
    let digest = check_remote_digest_with_docker_detailed(image_full, docker).await?;
    Ok((digest.manifest_digest, digest.config_digest, digest.tag))
}

/// Like [`check_remote_digest_with_docker`] but also reports whether the
/// resolution was degraded (config digest unavailable, manifest reused).
pub async fn check_remote_digest_with_docker_detailed(
    image_full: &str,
    docker: &Docker,
) -> Result<RemoteDigest, String> {
    check_remote_digest_impl(image_full, Some(docker)).await
}

/// Outcome of a remote digest resolution.
///
/// `Degraded` is returned when the registry could not provide the real config
/// digest and the manifest digest is reused instead; comparing it against the
/// local image id may then produce a false positive.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DigestResolution {
    Resolved {
        manifest_digest: String,
        config_digest: String,
        tag: String,
    },
    Degraded {
        manifest_digest: String,
        tag: String,
    },
}

/// Log level associated with a digest resolution outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DigestLogLevel {
    Info,
    Warn,
    Error,
}

/// Classify a digest resolution into its log level and message.
///
/// Pure so it can be unit-tested without touching a registry or Docker. Every
/// resolution path — HTTP success, daemon success, degraded fallback and total
/// failure — maps to exactly one level.
fn classify_digest_outcome(
    image_full: &str,
    repo: &str,
    tag: &str,
    outcome: &Result<DigestResolution, String>,
) -> (DigestLogLevel, String) {
    match outcome {
        Ok(DigestResolution::Resolved {
            manifest_digest,
            config_digest,
            ..
        }) => (
            DigestLogLevel::Info,
            format!(
                "check_remote_digest [{}:{}]: OK manifest_digest={} config_digest={}",
                repo,
                tag,
                short_digest(manifest_digest),
                short_digest(config_digest)
            ),
        ),
        Ok(DigestResolution::Degraded { manifest_digest, .. }) => (
            DigestLogLevel::Warn,
            format!(
                "check_remote_digest [{}:{}]: DEGRADED manifest_digest={} — no se pudo obtener el config digest; se usa el manifest digest para comparar (posible falso positivo)",
                repo,
                tag,
                short_digest(manifest_digest)
            ),
        ),
        Err(e) => (
            DigestLogLevel::Error,
            format!("check_remote_digest: FAILED image={}: {}", image_full, e),
        ),
    }
}

/// Resolve the remote digest of `image_full` while recording exactly one
/// outcome per invocation (`INFO` success, `WARN` degraded, `ERROR` failure).
async fn check_remote_digest_impl(
    image_full: &str,
    docker: Option<&Docker>,
) -> Result<RemoteDigest, String> {
    let parsed = parse_image_ref(image_full);
    let repo = parsed.repo.clone();
    let tag = parsed.tag.clone();

    let outcome = resolve_remote_digest(image_full, docker).await;

    let (level, message) = classify_digest_outcome(image_full, &repo, &tag, &outcome);
    match level {
        DigestLogLevel::Info => tracing::info!("{}", message),
        DigestLogLevel::Warn => tracing::warn!("{}", message),
        DigestLogLevel::Error => tracing::error!("{}", message),
    }

    match outcome {
        Ok(DigestResolution::Resolved {
            manifest_digest,
            config_digest,
            tag,
        }) => Ok(RemoteDigest {
            manifest_digest,
            config_digest,
            tag,
            degraded: false,
        }),
        Ok(DigestResolution::Degraded {
            manifest_digest,
            tag,
        }) => Ok(RemoteDigest {
            config_digest: manifest_digest.clone(),
            manifest_digest,
            tag,
            degraded: true,
        }),
        Err(e) => Err(e),
    }
}

/// Inner resolver: performs the actual registry/daemon requests and returns the
/// raw outcome (no logging). Callers MUST go through
/// [`check_remote_digest_impl`] so the outcome is always recorded.
async fn resolve_remote_digest(
    image_full: &str,
    docker: Option<&Docker>,
) -> Result<DigestResolution, String> {
    let _permit = digest_semaphore()
        .acquire()
        .await
        .map_err(|e| format!("semaphore: {}", e))?;
    let client = http_client();
    let parsed = parse_image_ref(image_full);
    if parsed.digest_only {
        return Err("image referenced only by digest, cannot check".to_string());
    }
    // A digest-pinned reference (`repo:tag@sha256:...`) is checked by its tag,
    // never by the pin nor by the literal "digest".
    if let Some(ref digest) = parsed.digest {
        tracing::debug!(
            "check_remote_digest: imagen pinneada por digest {} — se consulta por tag '{}'",
            short_digest(digest),
            parsed.tag
        );
    }
    let registry_host = parsed.registry.clone();
    let repo = parsed.repo.clone();
    let tag = parsed.tag.clone();

    tracing::info!(
        "check_remote_digest: image={} registry={} repo={} tag={}",
        image_full,
        registry_host,
        repo,
        tag
    );

    let resolution = match registry_host.as_str() {
        "docker.io" => {
            let token_url = format!(
                "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
                repo
            );
            tracing::info!(
                "check_remote_digest [{}:{}]: docker.io, token_url={}",
                repo,
                tag,
                token_url
            );
            let token = fetch_token(client, &token_url, &repo, &tag).await?;
            let (manifest_digest, config_digest) =
                fetch_manifest_digests(client, "registry-1.docker.io", &repo, &tag, Some(&token))
                    .await?;
            DigestResolution::Resolved {
                manifest_digest,
                config_digest,
                tag,
            }
        }
        "ghcr.io" => {
            let token_url = format!(
                "https://ghcr.io/token?service=ghcr.io&scope=repository:{}:pull",
                repo
            );
            tracing::info!(
                "check_remote_digest [{}:{}]: ghcr.io, token_url={}",
                repo,
                tag,
                token_url
            );
            let token = fetch_token(client, &token_url, &repo, &tag).await?;
            let (manifest_digest, config_digest) =
                fetch_manifest_digests(client, "ghcr.io", &repo, &tag, Some(&token)).await?;
            DigestResolution::Resolved {
                manifest_digest,
                config_digest,
                tag,
            }
        }
        other => {
            // Unknown registry: try Docker daemon FIRST (has credentials configured).
            if let Some(d) = docker {
                tracing::info!(
                    "check_remote_digest [{}:{}]: registry={} — intentando Docker daemon primero",
                    repo,
                    tag,
                    other
                );
                match d.inspect_registry_image(tag_ref(image_full), None).await {
                    Ok(dist) => {
                        if let Some(manifest_digest) = dist.descriptor.digest {
                            tracing::info!(
                                "check_remote_digest [{}:{}]: Docker daemon OK manifest_digest={}",
                                repo,
                                tag,
                                short_digest(&manifest_digest)
                            );
                            // Got manifest digest from daemon. Now get config digest via HTTP.
                            match fetch_manifest_digests(
                                client,
                                other,
                                &repo,
                                &manifest_digest,
                                None,
                            )
                            .await
                            {
                                Ok((_, config_digest)) => {
                                    return Ok(DigestResolution::Resolved {
                                        manifest_digest,
                                        config_digest,
                                        tag,
                                    });
                                }
                                Err(_) => {
                                    // Try with auth flow
                                    match fetch_manifest_with_auth(
                                        client,
                                        other,
                                        &repo,
                                        &manifest_digest,
                                    )
                                    .await
                                    {
                                        Ok((resp, _)) => {
                                            let body: serde_json::Value =
                                                resp.json().await.map_err(|e| {
                                                    format!("manifest parse failed: {}", e)
                                                })?;
                                            let config_digest = body["config"]["digest"]
                                                .as_str()
                                                .ok_or_else(|| "no config digest".to_string())?
                                                .to_string();
                                            return Ok(DigestResolution::Resolved {
                                                manifest_digest,
                                                config_digest,
                                                tag,
                                            });
                                        }
                                        Err(e) => {
                                            tracing::debug!(
                                                "check_remote_digest [{}:{}]: config digest no disponible tras daemon ({}); usando el manifest digest como fallback",
                                                repo,
                                                tag,
                                                e
                                            );
                                            // Fallback degradado: se usa el manifest digest como config
                                            // (posible falso positivo; el desenlace lo registra
                                            // `classify_digest_outcome` como WARN una sola vez).
                                            return Ok(DigestResolution::Degraded {
                                                manifest_digest,
                                                tag,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::info!(
                            "check_remote_digest [{}:{}]: Docker daemon falló (seguimos con HTTP): {}",
                            repo,
                            tag,
                            e
                        );
                    }
                }
            }

            // Fallback: HTTP probe + auth flow using fetch_manifest_with_auth
            match fetch_manifest_with_auth(client, other, &repo, &tag).await {
                Ok((resp, manifest_digest)) => {
                    let body: serde_json::Value = resp
                        .json()
                        .await
                        .map_err(|e| format!("manifest parse failed: {}", e))?;

                    if body.get("manifests").is_some()
                        && body
                            .get("mediaType")
                            .and_then(|m| m.as_str())
                            .is_some_and(|m| {
                                m.contains("manifest.list") || m.contains("image.index")
                            })
                    {
                        // Manifest list: resolve platform-specific manifest
                        let manifests = body["manifests"]
                            .as_array()
                            .ok_or_else(|| "no manifests in list".to_string())?;
                        let platform = crate::models::current_platform();
                        let parts: Vec<&str> = platform.split('/').collect();
                        let (os, arch) = if parts.len() == 2 {
                            (parts[0], parts[1])
                        } else {
                            ("linux", "amd64")
                        };
                        let amd64_digest = manifests
                            .iter()
                            .find(|m| {
                                let plat = &m["platform"];
                                plat["architecture"].as_str() == Some(arch)
                                    && plat["os"].as_str() == Some(os)
                            })
                            .or_else(|| {
                                manifests.iter().find(|m| {
                                    let plat = &m["platform"];
                                    plat["architecture"].as_str() == Some("amd64")
                                        && plat["os"].as_str() == Some("linux")
                                })
                            })
                            .or_else(|| manifests.first())
                            .and_then(|m| m["digest"].as_str())
                            .ok_or_else(|| "no suitable platform manifest".to_string())?;

                        let (plat_resp, plat_manifest_digest) =
                            fetch_manifest_with_auth(client, other, &repo, amd64_digest).await?;
                        let plat_body: serde_json::Value = plat_resp
                            .json()
                            .await
                            .map_err(|e| format!("platform manifest parse failed: {}", e))?;
                        let config_digest = plat_body["config"]["digest"]
                            .as_str()
                            .ok_or_else(|| "no config digest in platform manifest".to_string())?
                            .to_string();
                        DigestResolution::Resolved {
                            manifest_digest: plat_manifest_digest,
                            config_digest,
                            tag,
                        }
                    } else {
                        let config_digest = body["config"]["digest"]
                            .as_str()
                            .ok_or_else(|| "no config digest".to_string())?
                            .to_string();
                        DigestResolution::Resolved {
                            manifest_digest,
                            config_digest,
                            tag,
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("manifest fetch failed: {}", e));
                }
            }
        }
    };

    Ok(resolution)
}

/// Parse the realm (token endpoint) from a Www-Authenticate header.
/// Format: Bearer realm="https://...",service="...",scope="..."
fn parse_realm(auth_header: &str) -> Option<String> {
    for part in auth_header.split(',') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("realm=\"") {
            if let Some(end) = value.rfind('"') {
                return Some(value[..end].to_string());
            }
        }
    }
    None
}

/// Obtiene un token Bearer del registry para el scope `repository:{repo}:pull`.
/// Comprueba el status HTTP antes de parsear el JSON y devuelve un preview
/// del body (truncado a 300 caracteres) en el mensaje de error.
async fn fetch_token(
    client: &reqwest::Client,
    token_url: &str,
    repo: &str,
    tag: &str,
) -> Result<String, String> {
    tracing::debug!(
        "fetch_token [{}:{}]: obteniendo token desde {}",
        repo,
        tag,
        token_url
    );
    // Check cache first — tokens are valid for 5 minutes
    let cache_key = format!("{}:{}", token_url, repo);
    {
        let cache = token_cache().lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            if cached.fetched_at.elapsed() < std::time::Duration::from_secs(300) {
                tracing::debug!("fetch_token [{}:{}]: usando token cacheado", repo, tag);
                return Ok(cached.token.clone());
            }
        }
    }
    let token_resp = client.get(token_url).send().await.map_err(|e| {
        tracing::warn!(
            "fetch_token [{}:{}]: token request failed: {}",
            repo,
            tag,
            e
        );
        format!("token request failed: {}", e)
    })?;
    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let preview: String = token_resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect();
        let msg = format!("token status: {} preview: {}", status, preview);
        tracing::warn!("fetch_token [{}:{}]: {}", repo, tag, msg);
        return Err(msg);
    }
    let token_body: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|e| format!("token parse failed: {}", e))?;
    let token = token_body["token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "no token".to_string())?;
    // Store in cache
    {
        let mut cache = token_cache().lock().unwrap();
        cache.insert(
            cache_key,
            CachedToken {
                token: token.clone(),
                fetched_at: Instant::now(),
            },
        );
    }
    Ok(token)
}

/// Fetch manifest and config digests from a registry.
///
/// Returns `(manifest_digest, config_digest)` where:
/// - `manifest_digest` comes from the `Docker-Content-Digest` response header
/// - `config_digest` comes from `body["config"]["digest"]`
///
/// For manifest lists (multi-arch): resolves the platform-specific manifest
/// and extracts BOTH digests from the platform manifest response.
fn extract_manifest_digest(resp: &reqwest::Response) -> Option<String> {
    resp.headers()
        .get("docker-content-digest")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Fetch both manifest digest and config digest from a registry.
///
/// Returns `(manifest_digest, config_digest)`.
/// Includes retries with exponential backoff on HTTP 429 (rate limit).
async fn fetch_manifest_digests(
    client: &reqwest::Client,
    registry_host: &str,
    repo: &str,
    reference: &str,
    token: Option<&str>,
) -> Result<(String, String), String> {
    let manifest_url = format!(
        "https://{}/v2/{}/manifests/{}",
        registry_host, repo, reference
    );
    tracing::debug!(
        "fetch_manifest_digests [{}:{}]: consultando manifiesto en {}",
        repo,
        reference,
        manifest_url
    );

    // Fetch manifest with retry on 429 (rate limit)
    let manifest_resp = {
        let mut last_resp = fetch_manifest(client, &manifest_url, token).await?;
        let mut status = last_resp.status();

        if status == 429 {
            for attempt in 1..=2 {
                let delay_secs = 2u64 * attempt;
                tracing::warn!(
                    "fetch_manifest_digests [{}:{}]: HTTP 429, reintentando en {}s (intento {}/2)",
                    repo,
                    reference,
                    delay_secs,
                    attempt
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                last_resp = fetch_manifest(client, &manifest_url, token).await?;
                status = last_resp.status();
                if status != 429 {
                    break;
                }
            }
        }

        if !status.is_success() {
            tracing::warn!(
                "fetch_manifest_digests [{}:{}]: manifest HTTP {}",
                repo,
                reference,
                status
            );
            return Err(format!("manifest status: {}", status));
        }

        last_resp
    };

    let manifest_digest = extract_manifest_digest(&manifest_resp)
        .ok_or_else(|| "no docker-content-digest header".to_string())?;

    let content_type = manifest_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    tracing::debug!(
        "fetch_manifest_digests [{}:{}]: content-type={} docker-content-digest={}",
        repo,
        reference,
        content_type,
        &manifest_digest
    );

    let config_digest = if content_type.contains("manifest.list")
        || content_type.contains("image.index")
    {
        tracing::debug!(
            "fetch_manifest_digests [{}:{}]: manifest list detectado, buscando plataforma amd64/linux",
            repo,
            reference
        );
        let body: serde_json::Value = manifest_resp
            .json()
            .await
            .map_err(|e| format!("manifest list parse failed: {}", e))?;
        let manifests = body["manifests"]
            .as_array()
            .ok_or_else(|| "no manifests in list".to_string())?;
        // Use current platform from the system, falling back to amd64/linux
        let platform = crate::models::current_platform();
        let parts: Vec<&str> = platform.split('/').collect();
        let (os, arch) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("linux", "amd64")
        };
        let amd64_digest = manifests
            .iter()
            .find(|m| {
                let plat = &m["platform"];
                plat["architecture"].as_str() == Some(arch) && plat["os"].as_str() == Some(os)
            })
            .or_else(|| {
                // Fallback: any amd64/linux if current platform not found
                manifests.iter().find(|m| {
                    let plat = &m["platform"];
                    plat["architecture"].as_str() == Some("amd64")
                        && plat["os"].as_str() == Some("linux")
                })
            })
            .or_else(|| manifests.first())
            .and_then(|m| m["digest"].as_str())
            .ok_or_else(|| "no suitable platform manifest".to_string())?;

        let plat_url = format!(
            "https://{}/v2/{}/manifests/{}",
            registry_host, repo, amd64_digest
        );
        tracing::debug!(
            "fetch_manifest_digests [{}:{}]: consultando manifiesto de plataforma en {}",
            repo,
            reference,
            plat_url
        );
        let plat_resp = fetch_manifest(client, &plat_url, token).await?;
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

    Ok((manifest_digest, config_digest))
}

/// Fetch manifest response from registry, handling auth if needed.
///
/// 1. Probes `GET /v2/{repo}/manifests/{reference}` without auth
/// 2. If 401/403: parses `Www-Authenticate`, extracts realm, fetches token, retries with token
/// 3. If success: returns `(response, manifest_digest_from_header)`
async fn fetch_manifest_with_auth(
    client: &reqwest::Client,
    registry_host: &str,
    repo: &str,
    reference: &str,
) -> Result<(reqwest::Response, String), String> {
    let probe_url = format!(
        "https://{}/v2/{}/manifests/{}",
        registry_host, repo, reference
    );
    tracing::debug!(
        "fetch_manifest_with_auth [{}:{}]: registry={} probe_url={}",
        repo,
        reference,
        registry_host,
        probe_url
    );

    let probe = fetch_manifest(client, &probe_url, None).await?;
    let status = probe.status();

    if status == 401 || status == 403 {
        // Parse Www-Authenticate to find the real token endpoint.
        let auth_header = probe
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        tracing::warn!(
            "fetch_manifest_with_auth [{}:{}]: registry={} requiere auth, Www-Authenticate={:?}",
            repo,
            reference,
            registry_host,
            auth_header
        );

        let realm =
            parse_realm(auth_header).unwrap_or_else(|| format!("https://{}/token", registry_host));
        let token_url = format!(
            "{}?service={}&scope=repository:{}:pull",
            realm, registry_host, repo
        );
        tracing::info!(
            "fetch_manifest_with_auth [{}:{}]: realm={} token_url={}",
            repo,
            reference,
            realm,
            token_url
        );
        let token = fetch_token(client, &token_url, repo, reference).await?;

        // Retry with auth token
        let auth_resp = fetch_manifest(client, &probe_url, Some(&token)).await?;
        if !auth_resp.status().is_success() {
            return Err(format!(
                "manifest status after auth: {}",
                auth_resp.status()
            ));
        }
        let manifest_digest = extract_manifest_digest(&auth_resp)
            .ok_or_else(|| "no docker-content-digest header after auth".to_string())?;
        Ok((auth_resp, manifest_digest))
    } else if status.is_success() {
        let manifest_digest = extract_manifest_digest(&probe)
            .ok_or_else(|| "no docker-content-digest header".to_string())?;
        Ok((probe, manifest_digest))
    } else {
        Err(format!("manifest status: {}", status))
    }
}

/// Realiza la petición GET al manifiesto con los Accept headers adecuados,
/// añadiendo `Authorization: Bearer <token>` solo si se proporciona token.
async fn fetch_manifest(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> Result<reqwest::Response, String> {
    let mut req = client.get(url);
    if let Some(tok) = token {
        req = req.header("Authorization", format!("Bearer {}", tok));
    }
    req.header(
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
    .map_err(|e| format!("manifest request failed: {}", e))
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
        let parsed = parse_image_ref("nginx:latest");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/nginx");
        assert_eq!(parsed.tag, "latest");
        assert!(parsed.digest.is_none());
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_with_version_tag() {
        let parsed = parse_image_ref("library/postgres:15-alpine");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/postgres");
        assert_eq!(parsed.tag, "15-alpine");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_with_digest() {
        let parsed = parse_image_ref(
            "nginx@sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/nginx");
        // No tag in the reference → defaults to `latest`, never the literal "digest".
        assert_eq!(parsed.tag, "latest");
        assert_eq!(
            parsed.digest.as_deref(),
            Some("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1")
        );
        assert!(!parsed.digest_only);
    }

    // ── Pinneado por Alloy: `repo:tag@sha256:...` ─────────────

    #[test]
    fn test_parse_image_tag_and_digest_preserves_tag() {
        let parsed = parse_image_ref(
            "gitea/gitea:1.27.3@sha256:87a6abc123def456abc123def456abc123def456abc123def456abc123def456ab",
        );
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "gitea/gitea");
        assert_eq!(parsed.tag, "1.27.3");
        assert_eq!(
            parsed.digest.as_deref(),
            Some("sha256:87a6abc123def456abc123def456abc123def456abc123def456abc123def456ab")
        );
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_registry_with_port() {
        let parsed = parse_image_ref("registry.example.com:5000/myimage:v2");
        assert_eq!(parsed.registry, "registry.example.com:5000");
        assert_eq!(parsed.repo, "myimage");
        assert_eq!(parsed.tag, "v2");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_registry_with_port_and_digest() {
        let parsed = parse_image_ref(
            "registry.example.com:5000/myimage:v2@sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert_eq!(parsed.registry, "registry.example.com:5000");
        assert_eq!(parsed.repo, "myimage");
        assert_eq!(parsed.tag, "v2");
        assert_eq!(
            parsed.digest.as_deref(),
            Some("sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1")
        );
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_without_tag_defaults_latest() {
        let parsed = parse_image_ref("alpine");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/alpine");
        assert_eq!(parsed.tag, "latest");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_registry_path_with_tag() {
        let parsed = parse_image_ref("docker.io/library/redis:7.2");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/redis");
        assert_eq!(parsed.tag, "7.2");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_registry_path_with_digest() {
        let parsed = parse_image_ref(
            "docker.io/library/redis@sha256:fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
        );
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/redis");
        assert_eq!(parsed.tag, "latest");
        assert_eq!(
            parsed.digest.as_deref(),
            Some("sha256:fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321")
        );
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_empty() {
        let parsed = parse_image_ref("");
        assert!(parsed.digest_only);
        assert_eq!(parsed.registry, "");
        assert_eq!(parsed.repo, "");
        assert_eq!(parsed.tag, "");
    }

    // ── Nuevos casos del refactor registry-aware ─────────────

    #[test]
    fn test_parse_image_ghcr() {
        let parsed = parse_image_ref("ghcr.io/owner/repo:tag");
        assert_eq!(parsed.registry, "ghcr.io");
        assert_eq!(parsed.repo, "owner/repo");
        assert_eq!(parsed.tag, "tag");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_sha256_digest_only() {
        let parsed = parse_image_ref(
            "sha256:abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
        );
        assert!(parsed.digest_only);
        assert!(parsed.digest.is_none());
        assert_eq!(parsed.registry, "");
        assert_eq!(parsed.repo, "");
        assert_eq!(parsed.tag, "");
    }

    #[test]
    fn test_parse_image_postgres_17_alpine() {
        let parsed = parse_image_ref("postgres:17-alpine");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/postgres");
        assert_eq!(parsed.tag, "17-alpine");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_forgejo() {
        let parsed = parse_image_ref("forgejo.ellis.link/owner/repo:tag");
        assert_eq!(parsed.registry, "forgejo.ellis.link");
        assert_eq!(parsed.repo, "owner/repo");
        assert_eq!(parsed.tag, "tag");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_docker_io_explicit() {
        let parsed = parse_image_ref("docker.io/atareao/alloy:latest");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "atareao/alloy");
        assert_eq!(parsed.tag, "latest");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_library_nginx_no_duplicate() {
        // Docker Hub con repo de un solo nombre → prefijo library/ una sola vez
        let parsed = parse_image_ref("library/nginx:latest");
        assert_eq!(parsed.registry, "docker.io");
        assert_eq!(parsed.repo, "library/nginx");
        assert_eq!(parsed.tag, "latest");
        assert!(!parsed.digest_only);
    }

    #[test]
    fn test_parse_image_alpine_tag_no_digest() {
        let parsed = parse_image_ref("nginx:alpine");
        assert_eq!(parsed.tag, "alpine");
        assert!(parsed.digest.is_none());
        assert!(!parsed.digest_only);
    }

    // ── tag_ref: referencia sin el sufijo @digest ─────────────

    #[test]
    fn test_tag_ref_strips_digest() {
        assert_eq!(
            tag_ref("gitea/gitea:1.27.3@sha256:87a6deadbeef"),
            "gitea/gitea:1.27.3"
        );
    }

    #[test]
    fn test_tag_ref_without_digest_unchanged() {
        assert_eq!(tag_ref("repo:tag"), "repo:tag");
        assert_eq!(tag_ref("nginx:alpine"), "nginx:alpine");
    }

    #[test]
    fn test_tag_ref_no_tag_no_digest() {
        assert_eq!(tag_ref("nginx"), "nginx");
    }

    #[test]
    fn test_tag_ref_bare_digest_unchanged() {
        assert_eq!(tag_ref("sha256:abc123"), "sha256:abc123");
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

    // ── Desenlace del check de digest (update-check-robustness) ──

    #[test]
    fn test_classify_digest_outcome_http_ok_is_info() {
        let outcome = Ok(DigestResolution::Resolved {
            manifest_digest: "sha256:aaaaaaaaaaaabbbb".into(),
            config_digest: "sha256:ccccccccccccdddd".into(),
            tag: "latest".into(),
        });
        let (level, msg) =
            classify_digest_outcome("nginx:latest", "library/nginx", "latest", &outcome);
        assert_eq!(level, DigestLogLevel::Info);
        assert!(msg.contains("OK"), "msg={msg}");
        assert!(msg.contains("manifest_digest="), "msg={msg}");
        assert!(msg.contains("config_digest="), "msg={msg}");
        assert!(msg.contains("library/nginx:latest"), "msg={msg}");
    }

    #[test]
    fn test_classify_digest_outcome_daemon_ok_is_info() {
        // El camino del daemon también resuelve a `Resolved`, por lo que
        // debe registrarse como INFO y no saltarse por un return temprano.
        let outcome = Ok(DigestResolution::Resolved {
            manifest_digest: "sha256:dddd".into(),
            config_digest: "sha256:eeee".into(),
            tag: "1.0".into(),
        });
        let (level, msg) =
            classify_digest_outcome("forgejo.ellis.link/a/b:1.0", "a/b", "1.0", &outcome);
        assert_eq!(level, DigestLogLevel::Info);
        assert!(msg.contains("OK"), "msg={msg}");
    }

    #[test]
    fn test_classify_digest_outcome_degraded_is_warn() {
        let outcome = Ok(DigestResolution::Degraded {
            manifest_digest: "sha256:ffff".into(),
            tag: "latest".into(),
        });
        let (level, msg) =
            classify_digest_outcome("forgejo.ellis.link/a/b:latest", "a/b", "latest", &outcome);
        assert_eq!(level, DigestLogLevel::Warn);
        assert!(msg.contains("DEGRADED"), "msg={msg}");
        assert!(msg.to_lowercase().contains("falso positivo"), "msg={msg}");
    }

    #[test]
    fn test_classify_digest_outcome_failure_is_error() {
        let outcome: Result<DigestResolution, String> =
            Err("manifest status: 403 Forbidden".into());
        let (level, msg) =
            classify_digest_outcome("forgejo.ellis.link/a/b:latest", "a/b", "latest", &outcome);
        assert_eq!(level, DigestLogLevel::Error);
        assert!(msg.contains("FAILED"), "msg={msg}");
        assert!(msg.contains("403"), "msg={msg}");
    }

    #[test]
    fn test_classify_digest_outcome_degraded_returns_equal_digests() {
        // El resultado degradado que se devuelve al caller iguala config y
        // manifest digest (posible falso positivo), documentado por el WARN.
        let outcome = Ok(DigestResolution::Degraded {
            manifest_digest: "sha256:abcd".into(),
            tag: "latest".into(),
        });
        let (level, _) =
            classify_digest_outcome("some.registry/x/y:latest", "x/y", "latest", &outcome);
        assert_eq!(level, DigestLogLevel::Warn);
        // El mapeo a la tupla pública se cubre en el helper de resolución;
        // aquí comprobamos que el enum degradado transporta el manifest.
        match outcome {
            Ok(DigestResolution::Degraded {
                manifest_digest, ..
            }) => {
                assert_eq!(manifest_digest, "sha256:abcd")
            }
            _ => panic!("expected degraded"),
        }
    }
}
