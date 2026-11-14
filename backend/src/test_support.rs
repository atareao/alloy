//! Utilidades compartidas por los tests del backend.
//!
//! Este módulo solo se compila en builds de test (`#[cfg(test)]`). Centraliza la
//! resolución del socket del daemon de contenedores para que ningún módulo de test
//! duplique la lógica ni asuma un UID concreto.

use bollard::{Docker, API_DEFAULT_VERSION};

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
}
