# Proposal

## Why

El análisis de los logs de un ciclo completo de `update-check` (cron `0 6 * * *`,
ejecutado a las 04:00 UTC) revela cuatro defectos en la ruta de actualizaciones del
scheduler, ninguno de los cuales produce un log de nivel `ERROR`:

1. **`PullRestartStack` es un no-op silencioso.** `workers/scheduler.rs` ejecuta
   `tokio::process::Command::new("docker")` para `compose pull` / `up -d`, pero la imagen
   runtime (`Dockerfile:50-52`) solo instala `ca-certificates`; **no hay CLI de Docker**.
   Además `resolve_compose_file` filtra por `Path::new(p).exists()` y la ruta del compose
   del host no suele ser visible dentro del contenedor. El `match` solo contempla
   `Ok(..) if success` y un `_ => {}` que no loguea nada. Resultado: Alloy detecta su
   propia actualización (`check_remote_digest [atareao/alloy:latest]: OK`) y **nunca la
   aplica**, sin error ni notificación.

2. **Errores de digest tragados y estado pisado.** `scheduler.rs` usa `Err(_) => { ...;
   sqlite_update_has_update(false); continue; }`. El error se descarta sin log y un fallo
   transitorio de registry marca el contenedor como "sin actualización".

3. **Degradación silenciosa del check.** `digest.rs` devuelve `Ok` usando el manifest
   digest como si fuera config digest (`continuwuation/continuwuity` en
   `forgejo.ellis.link`: daemon OK, luego HTTP 403 y token 403). Además los caminos de
   fallback hacen `return Ok(...)` **antes** del `tracing::info!("... OK ...")`, dejando
   sin desenlace registrado a registries como `public.ecr.aws/zinclabs/openobserve`.

4. **Comprobaciones duplicadas.** Varios contenedores comparten imagen y el registry se
   consulta una vez por contenedor (`wordpress:fpm-alpine` ×3, `nginx:alpine` ×3,
   `redis:8` ×3, `mariadb:latest` ×3, `atareao/mariadb-backup` ×3), alargando el ciclo a
   ~2m46s.

El efecto neto es que la automatización de actualizaciones es inobservable e incompleta:
no hay `ERROR`s, las políticas de stack no se aplican y no se puede distinguir un fallo
de red de una imagen al día.

## What Changes

- **`PullRestartStack` funcional y observable.** Reescribir la ruta para que no dependa
  del CLI `docker` en la imagen (usar la API vía Bollard para `pull`/recreate del
  servicio, o documentar/inyectar el CLI + plugin compose como requisito de imagen).
  Añadir logging `ERROR` en todos los caminos de fallo (binario ausente, compose file no
  resuelto, salida no-cero con stderr) y `INFO` en éxito. Emitir notificación como
  `PullRestart`. Marcar/limpiar `update_in_progress` de forma simétrica.
- **Errores de check visibles y sin pisar estado.** En el scheduler, loguear el error de
  `check_remote_digest_with_docker` (con nombre e imagen) y **no** forzar
  `has_update=false` ante un error de resolución remota.
- **Desenlace siempre registrado en el check de digest.** Un único punto de salida que
  loguee `OK`/`DEGRADED`/`FAILED`, incluyendo el fallback que iguala config y manifest
  digest (marcado explícitamente como no fiable).
- **Deduplicación por referencia de imagen.** Resolver una sola vez por imagen única
  dentro de un ciclo y reutilizar el resultado entre contenedores que la comparten.

## Capabilities

### New Capabilities
- `updates/update-check`: ejecución observable de políticas del scheduler
  (`PullRestartStack`), propagación de errores de resolución remota y deduplicación de
  comprobaciones de digest por imagen.

### Modified Capabilities
<!-- Ninguna: `updates/digest-comparison` mantiene sus requisitos; el nuevo capability
     cubre la observabilidad y el no-pisado de estado, no contradictorios con aquel. -->

## Impact

- `backend/src/workers/scheduler.rs`: rama `PullRestartStack`, manejo de `Err(_)` del
  check, deduplicación y `update_in_progress`.
- `backend/src/updates/digest.rs`: punto único de salida con log de desenlace y aviso de
  resultado degradado.
- `Dockerfile`: decisión de image runtime (instalar `docker-cli` + `docker-cli-compose`
  + `docker-cli-buildx`, o eliminar la dependencia del CLI).
- Posible `backend/src/stacks.rs` / `updates/handlers.rs` si se centraliza el recreate de
  stack vía API.
- Sin cambios de frontend. Sin cambios de schema SQLite.
- Fuera de alcance: política específica para imágenes pinneadas por digest (se documenta
  como comportamiento conocido; ver `updates/digest-pinned-refs`).
