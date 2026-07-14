# Alloy — Próximas features

## ✅ Backend (completado)

- [x] Webhooks de notificación (POST a URL arbitraria)
- [x] DELETE /api/images/{id} para eliminar imágenes
- [x] POST /api/images/prune para limpiar imágenes colgantes

## ✅ Frontend (completado)

- [x] Botón "Prune images" en ImagesPage
- [x] Vista de stacks con logs
- [x] Notificaciones toast persistentes vía SSE

## ✅ Refactor (completado)

- [x] workers.rs (931→397 líneas) — dividido en 4 submódulos
- [x] updates.rs (792→321 líneas) — dividido en 3 submódulos
- [x] thiserror para AppError

## ⏳ Testing (pendiente)

- [ ] Tests e2e con OIDC mock
- [ ] CI/CD: GitHub Actions
- [ ] Tests de check_remote_digest con wiremock

## 💡 Ideas

- [ ] Backup automático programado
- [ ] Dashboard con gráficas de uso
- [ ] Soporte multi-host
- [ ] Compatibilidad con Quadlets
