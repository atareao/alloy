# Design

## Context

El worker `update-check` (`backend/src/workers/scheduler.rs`) recorre todos los
contenedores, resuelve el digest remoto de cada imagen (`updates/digest.rs`) y aplica la
política configurada (`Pull`, `PullRestart`, `PullRestartStack`). La observación de un
ciclo real muestra: (1) `PullRestartStack` nunca aplica nada y no loguea errores; (2) los
`Err` del check se descartan y fuerzan `has_update=false`; (3) el check de digest tiene
`return` tempranos que omiten el log de desenlace y un fallback degradado mudo; (4) el
registry se consulta una vez por contenedor aunque la imagen se repita.

## Goals / Non-Goals

**Goals:**
- Que `PullRestartStack` actualice de verdad o falle con `ERROR` visible.
- Que ningún fallo de resolución remota se silencie ni pise el estado de actualización.
- Que todo check de digest tenga un desenlace logueado (OK / DEGRADED / FAILED).
- Que el registry se consulte como máximo una vez por imagen única y ciclo.

**Non-Goals:**
- Cambiar el algoritmo de comparación (`needs_update`) — sigue en `updates/digest-comparison`.
- Definir política nueva para imágenes pinneadas por digest.
- Cambiar el schema SQLite o el frontend.

## Decisions

### D1 (RESUELTA): `PullRestartStack` vía API Bollard
**Elegido: (a) recrear el servicio con la API de Docker (Bollard), sin depender del CLI
`docker` en la imagen runtime.**

Procedimiento:
1. Leer labels del contenedor del servicio: `com.docker.compose.project`,
   `com.docker.compose.service`, `com.docker.compose.project.config_files` y
   `com.docker.compose.project.working_dir`.
2. `docker.create_image` (pull) de la imagen del servicio por digest.
3. Recrear el contenedor respetando la config de compose: nombre, labels, redes, mounts y
   `depends_on` (best-effort; documentar límites frente a `docker compose up`).
4. Resolver el `image_id` del contenedor recreado y persistirlo como `last_remote_digest`.
5. Notificar (`notify_all`), marcar `has_update=false` e incrementar `updated_count`.
6. `ERROR` en cada fallo (pull, create, start, inspect) incluyendo la causa.

Consecuencia: **no** se instala `docker-cli` en la imagen y **no** se requiere que el
compose file sea visible dentro del contenedor. La pregunta de visibilidad del compose
file deja de ser bloqueante.

Límites asumidos: la recreación vía API no evalúa `depends_on` ni variables de entorno de
`.env` de compose. Se documentan como no-goals de este cambio.

### D2: Un único punto de salida en el check de digest
Refactorizar `check_remote_digest_impl` para que todos los caminos (HTTP directo, daemon,
fallback degradado) salgan por un mismo bloque que registre el desenlace. El resultado
degradado se modela explícitamente (p. ej. un flag o variante) para poder emitir `WARN`.

### D3: Deduplicación por imagen
Un `HashMap<String, DigestCheckOutcome>` por ciclo indexado por `image_full`. Si una
imagen ya se resolvió, se reutiliza el resultado para los contenedores restantes. Los
tokens ya están cacheados; esto elimina además las peticiones de manifest repetidas.

### D4: Errores no pisan estado
El `Err` del check se loguea y se hace `continue`, sin `sqlite_update_has_update(false)`.
Esto evita que un fallo transitorio marque como "al día" un contenedor desactualizado.

## Risks / Trade-offs

- [Riesgo] La deduplicación puede ocultar diferencias si dos contenedores usan el mismo
  `image_full` pero distinta plataforma corriendo en hosts distintos. Mitigación: el
  runtime es único (un solo host Docker).
- [Riesgo] La recreación vía API no evalúa `depends_on` ni variables de `.env` de
  compose. Mitigación: se documenta como no-goal; la política garantiza pull + recreate del
  servicio afectado.

## Migration Plan

Sin migración de datos. Cambio puramente de comportamiento del worker. Rollback =
revertir el commit; no hay estado persistido nuevo.

## Open Questions

- RESUELTA: `update_in_progress` se inserta de forma simétrica en `Pull`, `PullRestart` y
  `PullRestartStack`, con guardia RAII que limpia en éxito, error y panic.
- RESUELTA: el resultado degradado se propaga a los callers (`RemoteDigest.degraded`) y no
  se persiste el manifest como config digest; se usa el image id inspeccionado y un guard
  anti-bucle por manifest ya aplicado.
