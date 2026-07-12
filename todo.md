# TODO — Alloy

Estado actual del proyecto y próximos pasos.

## ✅ Completado
- [x] Refactor OIDC-only auth (PocketID)
- [x] Migración a archivos modulares (ya no es single-file)
- [x] GIT_FLOW.md + gitflow recipes en justfile
- [x] Hotfix v0.5.2 — icono actualización nunca se limpiaba
- [x] Mostrar nombre de imagen en lugar de hash sha256
- [x] **Unificar Stacks en Dashboard**
  - [x] `POST /api/stacks/{project}/down` — borrar stack
  - [x] Dashboard: header oscuro + menú stack-level (7 ops)
  - [x] Stack inspect modal + confirm delete
  - [x] Eliminar StacksPage.tsx + tab
- [x] **Fase 1: Tareas programadas con stacks + notificaciones**
  - [x] Modelos target_type, notify, cleanup
  - [x] Resolución de targets stack + docker compose pull/up -d
  - [x] Cleanup delete-old + notificaciones
  - [x] Frontend: SegmentedControl container/stack + acciones según tipo
- [x] **Fase 2: Rollback en tareas programadas**
  - [x] Helpers: tag_backup, verify_healthy, rollback_container
  - [x] Worker: branch cleanup="rollback" completo
  - [x] Frontend: selector cleanup con "Rollback si falla"
- [x] **Release v0.6.0** → main + tag + push registry
- [x] **Fase 1 limpieza**: eliminar terminal.rs, mover health_h → main.rs
- [x] **Fase 2 limpieza**: mover config/history handlers a config.rs / updates.rs
- [x] **Fase 3 limpieza**: extraer persistence.rs de workers.rs

## 📋 Pendientes
- [ ] Validar flujo OIDC completo
- [ ] Revisar estado frontend (React/Vite)
- [ ] Revisar Dockerfile multi-stage
- [ ] Verificar Quadlet de ejemplo

## 🔮 Ideas / Futuro
- Tests e2e con OIDC mock
- Tests unitarios frontend

## 📊 Backend — Estado actual
```
backend/src/
├── admin.rs          →    96L  (alertas + schedules)
├── auth.rs           →   480L  (OIDC + JWT + frontend serve)
├── config.rs         →   320L  (Config + config_handler)
├── containers.rs     →   420L  (solo containers)
├── events.rs         →    62L  (SSE)
├── main.rs           →   280L  (+ health_h + mod persistence)
├── models.rs         →   402L  (tipos)
├── notifications.rs  →   135L  (Telegram + Matrix)
├── persistence.rs    →   153L  (nuevo — load_json + JsonWriter)
├── stacks.rs         →   374L  (docker compose)
├── state.rs          →   216L  (AppState + FromRefs)
├── updates.rs        →   520L  (updates + history)
└── workers.rs        →   630L  (workers + rollback helpers)
```