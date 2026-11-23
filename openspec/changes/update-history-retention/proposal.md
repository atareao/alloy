# Proposal: update-history-retention

## Descripción

La tabla `update_history` de SQLite registra cada evento de actualización de contenedores y crece sin límite. En instalaciones de larga duración esto degrada el rendimiento de consultas, infla el fichero de base de datos y complica los backups. El `HANDOFF.md` lo lista como prioridad alta: **"Poda/retención de `update_history` (crecimiento sin límite)"**.

Este cambio introduce una política de retención configurable por días y un mecanismo automático de poda que elimina los registros antiguos sin afectar a las operaciones críticas del sistema.

## Objetivo

Acotar el crecimiento de `update_history` mediante una retención configurable en días y una poda automática periódica, manteniendo la integridad de los datos recientes y sin bloquear las operaciones de actualización en curso.

## Alcance

- Campo de configuración `update_history_retention_days` (por defecto `30`, `0` = retención ilimitada).
- Persistencia del valor en la configuración existente del proyecto (mismo mecanismo que el resto de ajustes).
- Endpoints para leer y actualizar la política de retención.
- Tarea de poda automática ejecutada desde el worker de fondo existente, con intervalo configurable.
- Poda manual bajo demanda (endpoint o acción desde la UI de configuración).
- Log estructurado del resultado de cada poda (`deleted` count, duración).
- UI mínima en ConfigPage para ajustar los días de retención y lanzar poda manual.

## Exclusiones

- **No** se cambia el esquema de `update_history` (columnas, índices existentes) salvo el índice necesario para la poda por `created_at`.
- **No** se implementa archivado/exportación de histórico a ficheros externos.
- **No** se implementa retención por contenedor individual: la política es global.
- **No** se tocan las alertas ni los health checks (no existen).
- **No** se modifica el flujo de actualización (`update` de contenedores) más allá de garantizar que la poda no interfiera.
- **No** se añade dependencia externa nueva de scheduler: se reutiliza el worker/cron existente.

## Impacto

- **Specs afectadas:** `updates` (nueva capability delta).
- **Backend (`backend/src/`):**
  - Módulo de persistencia de `update_history` (nueva función `prune_update_history`).
  - Capa de configuración (`AppState` / struct de settings) con el nuevo campo.
  - Handlers HTTP para los endpoints de retención.
  - Worker de fondo para la poda programada.
- **DB:** migración idempotente que asegura índice sobre `update_history(created_at)`.
- **Frontend (`frontend/src/`):** ConfigPage con control de días + botón de poda manual; cliente API correspondiente.
- **Riesgo:** bajo. Operación de borrado acotada por `created_at` e índice; ejecutada fuera de transacciones largas.
- **Compatibilidad:** retrocompatible. Valor por defecto `30` días; instalaciones existentes podan en la primera ejecución.

## Dependencias

- Worker/scheduler de fondo ya presente en el proyecto.
- Capa de configuración persistente existente (no se introduce almacén nuevo).
- Cliente SQLite (`sqlx`/`rusqlite` según el proyecto) ya en uso.
- `openspec/changes/update-history-retention/` creado con `openspec new change` (hecho).

## Tareas

- [ ] Añadir `update_history_retention_days` a la configuración con default `30` y validación (`>= 0`).
- [ ] Migración idempotente: índice `idx_update_history_created_at`.
- [ ] Implementar `prune_update_history(days) -> u64` en el repositorio.
- [ ] Añadir `GET /api/updates/retention` y `PUT /api/updates/retention`.
- [ ] Añadir `POST /api/updates/retention/prune` (poda manual).
- [ ] Integrar poda automática en el worker de fondo con intervalo configurable.
- [ ] Log estructurado de cada poda (deleted, duration_ms, retention_days).
- [ ] UI en ConfigPage: input de días + acción de poda manual + feedback.
- [ ] Tests RED → GREEN → REFACTOR (unidad repositorio, handlers, UI).
- [ ] Documentar en `docs/update-history-retention.md` y actualizar HANDOFF.
