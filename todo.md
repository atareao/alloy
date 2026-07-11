# TODO — Alloy

Estado actual del proyecto y próximos pasos.

## ✅ Completado
- [x] Refactor OIDC-only auth (PocketID)
- [x] Migración a archivos modulares (ya no es single-file)
- [x] GIT_FLOW.md + gitflow recipes en justfile

## 📋 Pendientes

### Backend
- [ ] Verificar que compile con `cargo clippy -- -D warnings`
- [ ] Verificar tests existentes
- [ ] Revisar estado de módulos (admin.rs, auth.rs, containers.rs, etc.)
- [ ] Validar flujo OIDC completo

### Frontend
- [ ] Revisar estado del frontend (React/Vite)
- [ ] Verificar que build funciona

### Docker/Infra
- [ ] Revisar Dockerfile multi-stage
- [ ] Verificar Quadlet de ejemplo

## 🔮 Ideas / Futuro
- Tests e2e con OIDC mock
- Tests unitarios frontend