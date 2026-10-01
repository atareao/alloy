# Tasks

## 1. RED — Tests que fallan

- [x] 1.1 `stacks.rs`: helper de bajada por API — dado un conjunto de contenedores de un
      proyecto, produce la secuencia stop+remove (test con doble/abstracción sin Docker).
- [x] 1.2 `stacks.rs`: la resolución del compose file ya no depende de CLI — un proyecto
      con labels válidas se procesa sin invocar proceso externo.
- [x] 1.3 `handlers.rs`: la rama `PullRestartStack` no referencia `Command::new("docker")`
      (test de no-regresión / grep-asert).
- [x] 1.4 `cargo test` en rojo con los legacy en verde.

## 2. GREEN — Implementación mínima

- [x] 2.1 `stacks.rs::update_stack_h`: sustituir pull+up por `pull_image` +
      `recreate_container` por servicio; persistir digest; notificar; resultado ok/error.
- [x] 2.2 `stacks.rs::down_stack_h`: stop + remove por contenedor del proyecto vía API.
- [x] 2.3 `handlers.rs::apply_single_policy` rama `PullRestartStack`: reutilizar el camino
      Bollard (idealmente un helper compartido con el scheduler).
- [x] 2.4 Eliminar `get_compose_projects`/`get_compose_project_path` y el fallback CLI;
      retirar `resolve_compose_file` (scheduler) y su re-export en `workers/mod.rs`.
- [x] 2.5 `cargo test` verde + `cargo check`.

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 `cargo fmt`
- [x] 3.2 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 `cargo test` sin regresiones
- [x] 3.4 Revisión con `rust-reviewer`

## 4. Archive

- [x] 4.1 `openspec validate stack-update-bollard --strict`
- [x] 4.2 `openspec archive stack-update-bollard --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/stack-update-bollard/`
