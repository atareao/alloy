# Tasks

## 0. Decisión de diseño previa (bloqueante)

- [x] 0.1 Decidir el mecanismo de `PullRestartStack`: (a) recrear el servicio vía API
      Bollard, o (b) instalar `docker-cli` + `docker-cli-compose` + `docker-cli-buildx`
      en la imagen runtime y montar el compose file. Documentar en `design.md`.
      → **DECISIÓN D1 = (a) API Bollard**, sin tocar el `Dockerfile`.
- [x] 0.2 Confirmar con el usuario el compromiso (a) vs (b) antes de implementar.

## 1. RED — Tests que fallan

- [x] 1.1 `scheduler.rs`: helper puro de resolución/dedup por imagen — mismo ref compartido
      por 3 contenedores → 1 sola entrada en el mapa de resultados (unit test sin Docker).
- [x] 1.2 `scheduler.rs`: helper de manejo de error de check — `Err` no fuerza
      `has_update=false` y produce log (cubierto con función pura + captura de tracing).
- [x] 1.3 `scheduler.rs`: construcción de comando/estrategia de stack — ausencia de
      binario produce `Err` clasificable (no `_ => {}` mudo).
- [x] 1.4 `digest.rs`: función de desenlace — éxito HTTP, éxito daemon, degradado y fallo
      producen cada uno su nivel y mensaje esperados (unit tests).
- [x] 1.5 Ejecutar `cargo test` y confirmar que los nuevos tests fallan con los legacy en
      verde. → RED confirmado: 28 errores de compilación por símbolos aún inexistentes.

## 2. GREEN — Implementación mínima

- [x] 2.1 `digest.rs`: extraer `check_remote_digest_impl` a un único punto de salida que
      loguee el desenlace; marcar el fallback config=manifest como DEGRADED (`WARN`).
- [x] 2.2 `scheduler.rs`: `Err(e)` del check → `tracing::warn!/error!` con nombre+imagen;
      NO llamar a `sqlite_update_has_update(false)`.
- [x] 2.3 `scheduler.rs`: deduplicar por `image_full` (mapa de resultados por ciclo);
      reutilizar `(has_update, config_digest, manifest_digest)` entre contenedores.
- [x] 2.4 `scheduler.rs`: implementar `PullRestartStack` según la decisión 0.1, con
      `ERROR` en pull/recreate fallido e `INFO` en éxito; notificar y persistir digest en
      éxito. (Recreación vía Bollard `recreate_container`, sin CLI docker.)
- [x] 2.5 `update_in_progress`: insertar al inicio de cada acción y limpiar en todos los
      caminos (éxito y error).
- [x] 2.6 Verificar la decisión 0.1 en la imagen (`Dockerfile`) si aplica. → N/A: D1 no
      requiere cambios en el `Dockerfile`.
- [x] 2.7 `cargo test` verde + `cargo check`.

## 3. REFACTOR — Limpieza y verificación

- [x] 3.1 `cargo fmt`
- [x] 3.2 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 3.3 `cargo test` sin regresiones (11 tests nuevos en verde; 5 fallos
      pre-existentes de integración que requieren Docker real, ajenos a este cambio).
- [x] 3.4 Revisión con `rust-reviewer`

## 4. Archive

- [x] 4.1 `openspec validate update-check-robustness --strict`
- [x] 4.2 `openspec archive update-check-robustness --yes` (pendiente de revisión)
- [x] 4.3 Verificar/eliminar `openspec/changes/update-check-robustness/`
