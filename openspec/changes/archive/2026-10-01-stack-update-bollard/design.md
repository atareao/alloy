# Design

## Context

Tras `update-check-robustness`, el worker del scheduler ya no usa CLI. Quedan cuatro usos
de `docker compose` en `stacks.rs` y uno en `handlers.rs` (`apply_single_policy`). La
imagen runtime no instala el CLI, así que todas esas rutas fallan.

## Goals / Non-Goals

**Goals:**
- Actualizar y bajar stacks usando solo la API de Docker.
- Un único camino de actualización de servicio, compartido con el scheduler.
- Que cualquier fallo sea visible en la respuesta de la API.

**Non-Goals:**
- Reproducir la semántica completa de `docker compose` (interpolación de `.env`,
  `depends_on`, redes/volúmenes con nombre).
- Cambios de frontend o de schema.

## Decisions

### D1: Reutilizar `pull_image` + `recreate_container`
Tanto `update_stack_h` como `apply_single_policy` usan los helpers existentes
(`crate::containers::pull_image`, `crate::updates::handlers::recreate_container`), igual que
el worker del scheduler. Se puede extraer un helper compartido para evitar duplicación.

### D2 (RESUELTA): `down_stack_h` = stop + remove + redes, sin volúmenes
`docker compose down` se aproxima con `stop_container` + `remove_container` por contenedor
del proyecto (label `com.docker.compose.project`), más `remove_network` de las redes del
proyecto. Si una red está en uso por un contenedor externo, `remove_network` puede fallar:
se tolera con `WARN` y se continúa. **No** se eliminan volúmenes con nombre (los datos se
preservan); `down -v` queda fuera de alcance.

### D3: Eliminar descubrimiento CLI
`get_compose_projects`/`get_compose_project_path` y el fallback de compose file se
eliminan: el descubrimiento por labels (`LABEL_COMPOSE_PROJECT`/`SERVICE`) ya es el camino
principal en `list_stacks_h`. `resolve_compose_file` queda muerto y se retira junto con su
re-export en `workers/mod.rs`.

## Risks / Trade-offs

- [Riesgo] `recreate_container` no evalúa `depends_on`; recrear un servicio puede dejar
  dependientes apuntando a un contenedor recreado. Mitigación: recrear por servicio y
  documentar; aceptable para el caso de uso.
- [Riesgo] `down` sin eliminar redes puede dejar redes huérfanas. Mitigación: documentar.
- [Riesgo] Tests de integración de stacks dependen de un socket Podman; los nuevos tests
  deben ser puros (sin Docker).

## Migration Plan

Sin migración de datos. Cambio de comportamiento. Rollback = revertir commit.

## Open Questions

- RESUELTA: `down_stack_h` = stop + remove de contenedores + eliminación de redes del
  proyecto; volúmenes con nombre preservados.
- RESUELTA: `update_stack_h` recrea por servicio (preserva `results[]` y `--no-deps`).
