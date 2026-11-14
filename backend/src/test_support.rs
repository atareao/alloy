//! Utilidades compartidas por los tests del backend.
//!
//! Este módulo solo se compila en builds de test (`#[cfg(test)]`). Centraliza la
//! resolución del socket del daemon de contenedores para que ningún módulo de test
//! duplique la lógica ni asuma un UID concreto.

use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::{Docker, API_DEFAULT_VERSION};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::OnceCell;

/// Socket del sistema (podman instalado a nivel root).
const SYSTEM_SOCKET: &str = "/run/podman/podman.sock";
/// Fallback histórico: sesión rootless del UID 1000.
const LEGACY_SOCKET: &str = "/run/user/1000/podman/podman.sock";

/// Resuelve la ruta del socket del daemon de contenedores.
///
/// Pura a propósito: recibe los valores de entorno y un predicado de existencia para
/// poder testearse sin tocar variables de entorno (proceso-global) ni el sistema de ficheros.
///
/// Prioridad: `DOCKER_HOST` → `${XDG_RUNTIME_DIR}/podman/podman.sock` (si existe) → socket del
/// sistema `/run/podman/podman.sock` (si existe) → fallback legacy.
pub fn resolve_socket(
    docker_host: Option<&str>,
    xdg_runtime_dir: Option<&str>,
    socket_exists: impl Fn(&str) -> bool,
) -> String {
    if let Some(host) = docker_host.filter(|h| !h.trim().is_empty()) {
        return host.trim().to_string();
    }
    if let Some(dir) = xdg_runtime_dir.filter(|d| !d.trim().is_empty()) {
        let candidate = format!("{}/podman/podman.sock", dir.trim().trim_end_matches('/'));
        if socket_exists(&candidate) {
            return candidate;
        }
    }
    if socket_exists(SYSTEM_SOCKET) {
        return SYSTEM_SOCKET.to_string();
    }
    LEGACY_SOCKET.to_string()
}

/// Socket realmente usado en este entorno.
pub fn podman_socket() -> String {
    let docker_host = std::env::var("DOCKER_HOST").ok();
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();
    resolve_socket(docker_host.as_deref(), xdg_runtime_dir.as_deref(), |path| {
        std::path::Path::new(path).exists()
    })
}

/// ¿`DOCKER_HOST` está definido o existe el `socket` dado?
///
/// Helper privado para evitar resolver el socket dos veces en [`podman_client`].
fn has_usable_socket(socket: &str) -> bool {
    std::env::var("DOCKER_HOST").is_ok_and(|h| !h.trim().is_empty())
        || std::path::Path::new(socket).exists()
}

/// Cliente `bollard` conectado al socket resuelto.
///
/// # Panics
/// Si no hay daemon accesible o la conexión falla, con un mensaje accionable.
pub fn podman_client() -> Docker {
    let socket = podman_socket();
    assert!(
        has_usable_socket(&socket),
        "Tests de integración: no hay daemon de contenedores accesible.\n  \
         Socket resuelto: {socket}\n  \
         Define DOCKER_HOST o arranca el socket (p. ej. `systemctl --user start podman.socket`)."
    );
    Docker::connect_with_local(&socket, 120, API_DEFAULT_VERSION).unwrap_or_else(|e| {
        panic!(
            "Tests de integración: falló la conexión al socket '{socket}': {e}.\n  \
             Comprueba que el daemon está en marcha y que DOCKER_HOST apunta a su socket."
        )
    })
}

/// Imagen mínima para los contenedores de fixture (misma base que el runtime del Dockerfile).
const FIXTURE_IMAGE: &str = "docker.io/library/alpine:3.23";
/// Contador global para nombres únicos entre tests paralelos.
static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Pull de la imagen de fixture una sola vez por proceso.
static FIXTURE_IMAGE_READY: OnceCell<()> = OnceCell::const_new();

/// Nombre único de fixture: `<prefix>-<pid>-<contador>`.
pub fn fixture_name(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

async fn ensure_fixture_image(docker: &bollard::Docker) {
    FIXTURE_IMAGE_READY
        .get_or_init(|| async {
            if docker.inspect_image(FIXTURE_IMAGE).await.is_ok() {
                return;
            }
            let mut stream = docker.create_image(
                Some(CreateImageOptions {
                    from_image: FIXTURE_IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            );
            while let Some(item) = stream.next().await {
                item.expect("Tests de integración: falló el pull de la imagen de fixture");
            }
        })
        .await;
}

/// Contenedor efímero de fixture. Se elimina (con `force`) al hacer `Drop`.
pub struct FixtureContainer {
    docker: bollard::Docker,
    name: String,
}

impl FixtureContainer {
    /// Crea y arranca un contenedor efímero con `<prefix>` como nombre base.
    ///
    /// Publica un puerto (`80/tcp` → puerto efímero del host) para que `fetch_containers`
    /// produzca `ports` no vacío, y usa `sleep 600` para que quede en estado `running`.
    pub async fn run(prefix: &str) -> Self {
        let docker = podman_client();
        ensure_fixture_image(&docker).await;
        let name = fixture_name(prefix);

        let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
        exposed_ports.insert("80/tcp".to_string(), HashMap::new());
        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
        port_bindings.insert(
            "80/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: None,
            }]),
        );

        let config = ContainerConfig {
            image: Some(FIXTURE_IMAGE.to_string()),
            cmd: Some(vec!["sleep".to_string(), "600".to_string()]),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                ..Default::default()
            }),
            ..Default::default()
        };

        docker
            .create_container(
                Some(CreateContainerOptions {
                    name: name.clone(),
                    platform: None,
                }),
                config,
            )
            .await
            .unwrap_or_else(|e| {
                panic!("Tests de integración: no se pudo crear el fixture '{name}': {e}")
            });

        // A partir de aquí el contenedor ya existe: se encapsula para que `Drop` lo limpie
        // aunque el arranque o la espera fallen (evita fixtures huérfanos).
        let fixture = FixtureContainer { docker, name };

        fixture
            .docker
            .start_container(fixture.name(), None::<StartContainerOptions<String>>)
            .await
            .unwrap_or_else(|e| {
                panic!(
                    "Tests de integración: no se pudo arrancar el fixture '{}': {e}",
                    fixture.name()
                )
            });

        // Espera a que el daemon lo registre (evita carrera con list_containers).
        for _ in 0..50 {
            let listed = fixture
                .docker
                .list_containers(Some(ListContainersOptions::<String> {
                    all: true,
                    ..Default::default()
                }))
                .await
                .unwrap_or_default();
            let found = listed.iter().any(|c| {
                c.names
                    .as_ref()
                    .map(|ns| {
                        ns.iter()
                            .any(|n| n.trim_start_matches('/') == fixture.name())
                    })
                    .unwrap_or(false)
            });
            if found {
                return fixture;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!(
            "Tests de integración: el fixture '{}' no apareció en list_containers",
            fixture.name()
        );
    }

    /// Nombre del contenedor de fixture.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for FixtureContainer {
    fn drop(&mut self) {
        let docker = self.docker.clone();
        let name = self.name.clone();
        // `remove_container` es async y no se puede `block_on` dentro del runtime de `#[tokio::test]`:
        // se ejecuta en un hilo con su propio runtime.
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("runtime de limpieza");
            rt.block_on(async {
                if let Err(e) = docker
                    .remove_container(
                        &name,
                        Some(RemoveContainerOptions {
                            force: true,
                            v: false,
                            link: false,
                        }),
                    )
                    .await
                {
                    tracing::warn!(
                        "Tests de integración: no se pudo eliminar el fixture '{name}': {e}"
                    );
                }
            });
        })
        .join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never_exists(_: &str) -> bool {
        false
    }

    fn only(path: &'static str) -> impl Fn(&str) -> bool {
        move |p| p == path
    }

    fn exists_in(paths: &[&'static str]) -> impl Fn(&str) -> bool {
        let paths = paths.to_vec();
        move |p| paths.contains(&p)
    }

    #[test]
    fn resolve_socket_docker_host_has_priority() {
        // DOCKER_HOST gana incluso si existe el socket del sistema.
        assert_eq!(
            resolve_socket(
                Some("unix:///custom/docker.sock"),
                Some("/run/user/1234"),
                only("/run/podman/podman.sock")
            ),
            "unix:///custom/docker.sock"
        );
    }

    #[test]
    fn resolve_socket_ignores_empty_docker_host() {
        // DOCKER_HOST vacío se ignora; el socket XDG (existente) gana.
        assert_eq!(
            resolve_socket(
                Some(""),
                Some("/run/user/1234"),
                only("/run/user/1234/podman/podman.sock")
            ),
            "/run/user/1234/podman/podman.sock"
        );
    }

    #[test]
    fn resolve_socket_prefers_xdg_runtime_dir() {
        // XDG definido y su socket existe: se usa el de XDG, aunque exista el del sistema.
        assert_eq!(
            resolve_socket(
                None,
                Some("/run/user/1234"),
                exists_in(&[
                    "/run/user/1234/podman/podman.sock",
                    "/run/podman/podman.sock"
                ])
            ),
            "/run/user/1234/podman/podman.sock"
        );

        // XDG definido pero sin socket: cae al socket del sistema si existe.
        assert_eq!(
            resolve_socket(
                None,
                Some("/run/user/1234"),
                exists_in(&["/run/podman/podman.sock"])
            ),
            "/run/podman/podman.sock"
        );
    }

    #[test]
    fn resolve_socket_uses_system_socket_when_it_exists() {
        assert_eq!(
            resolve_socket(None, None, only("/run/podman/podman.sock")),
            "/run/podman/podman.sock"
        );
    }

    #[test]
    fn resolve_socket_falls_back_to_legacy_path() {
        // Nada configurado y ningún socket del sistema: fallback legacy (UID 1000).
        assert_eq!(
            resolve_socket(None, None, never_exists),
            "/run/user/1000/podman/podman.sock"
        );
    }

    #[test]
    fn resolve_socket_ignores_empty_xdg_runtime_dir() {
        assert_eq!(
            resolve_socket(None, Some(""), never_exists),
            "/run/user/1000/podman/podman.sock"
        );
    }

    #[test]
    fn resolve_socket_normalizes_trailing_slash_in_xdg() {
        assert_eq!(
            resolve_socket(
                None,
                Some("/run/user/1234/"),
                exists_in(&["/run/user/1234/podman/podman.sock"])
            ),
            "/run/user/1234/podman/podman.sock"
        );
    }

    #[test]
    fn resolve_socket_ignores_whitespace_only_env_values() {
        // Solo espacios en blanco: se ignoran ambos y cae al fallback legacy.
        assert_eq!(
            resolve_socket(Some("   "), Some("   "), never_exists),
            "/run/user/1000/podman/podman.sock"
        );
    }

    #[test]
    fn fixture_name_is_unique_and_prefixed() {
        let a = fixture_name("alloy-it");
        let b = fixture_name("alloy-it");
        assert_ne!(a, b);
        assert!(a.starts_with("alloy-it-"));
        assert!(b.starts_with("alloy-it-"));
        assert!(a.contains(&std::process::id().to_string()));
    }
}
