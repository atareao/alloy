# Changelog
## [0.17.0] - 2026-07-18

### Bug Fixes

- Remove duplicate dashboard notifications rendered in App.tsx and DashboardPage

### Features

- Dashboard UI refinements (#12)
- Container logs SSE streaming with live viewer modal

### Other

- 0.16.0 (#13)

### Styling

- Mobile dashboard refinements, theme in ConfigPage, update docs (#14)
## [0.16.0] - 2026-07-18

### Bug Fixes

- Extraer PolicyActionButton a componente independiente, añadir política global
- Extraer PolicyActionButton a componente externo, eliminar import no usado

### Features

- Per-container update policies, default policy config, Matrix notifications on state changes
- Merge feature/container-policy-notifications into develop

### Other

- V0.16.0
## [0.15.0] - 2026-07-17

### Features

- Update-check cron, policy per container, navegación 3 vistas, eliminar CheckConfigPage

### Miscellaneous Tasks

- Bump version to 0.15.0

### Other

- Merge v0.14.0 into develop
- V0.15.0
## [0.14.0] - 2026-07-17

### Bug Fixes

- Sse-state-persistence
- Switch stopPropagation on monitoring toggle in dashboard header
- Merge switch stopPropagation
- Merge main into develop (switch stopPropagation)

### Features

- Stack menu táctil con botón en móvil y reorden de acciones
- Menú containers táctil con botón en móvil, reorden de acciones
- *(dashboard-persistence)* Dashboard redesign, backend persistence refactor, SQLite db module
- Merge dashboard-persistence into develop
- *(monitoring)* Per-container monitoring toggle with Switch in dashboard
- Merge monitoring-toggle into develop
- Remove AlertsPage — monitoring consolidado en Dashboard
- Update-check cron + políticas de actualización por contenedor
- Merge feature/remove-alerts-page into develop

### Miscellaneous Tasks

- Bump version to 0.14.0

### Other

- Merge v0.11.0 into develop
- V0.11.0
- V0.12.0
- V0.13.0
- V0.14.0
## [0.11.0] - 2026-07-15

### Bug Fixes

- Persistencia de estado SSE del dashboard al cambiar de pestaña
- Merge sse-state-persistence into develop
- Token visible, Matrix r0, dangling filter, cache en App

### Features

- Merge feature/fix/token-matrix-dangling-cache into develop

### Miscellaneous Tasks

- Bump version to 0.10.1
- Bump version to 0.11.0
## [0.10.4] - 2026-07-14

### Bug Fixes

- Notify_* ignora respuestas HTTP 4xx/5xx (errores silenciosos)
- Merge notify-http-status into develop
- Persistencia de estado SSE del dashboard al cambiar de pestaña
- Notify-http-status
## [0.10.3] - 2026-07-14

### Bug Fixes

- Anadir tracing al flujo de alertas y notificaciones para diagnosticar fallos
- Merge alerts-tracing into develop
- Notify_* ignora respuestas HTTP 4xx/5xx (errores silenciosos)
- Alerts-tracing
## [0.10.2] - 2026-07-14

### Bug Fixes

- Permisos data/, persistencia settings, rutas config y eliminacion StacksPage
- Merge matrix-settings-persistence into develop
- Anadir tracing al flujo de alertas y notificaciones para diagnosticar fallos
- Matrix-settings-persistence
## [0.10.1] - 2026-07-14

### Bug Fixes

- Build frontend - searchQuery state faltante y scrollArea en Modal
- Merge frontend-build-fixes into develop
- Permisos data/, persistencia settings, rutas config y eliminacion StacksPage
- Frontend-build-fixes
## [0.10.0] - 2026-07-14

### Features

- Export/import de configuración
- Stacks management, image prune, webhooks y refactor de submodulos
- Merge feature/stacks-prune-webhooks into develop

### Miscellaneous Tasks

- Bump version to 0.10.0
- Update Cargo.lock for v0.10.0

### Other

- Merge v0.9.0 into develop
- V0.10.0
## [0.9.0] - 2026-07-13

### Bug Fixes

- Restaurar ContainerImport en events.rs para tests

### Features

- Tests masivos, vista de imágenes y filtros en dashboard
- Merge feature/tests-images-dashboard into develop

### Miscellaneous Tasks

- Resolve Cargo.lock merge conflict
- Bump version to 0.9.0

### Other

- V0.8.0
- V0.9.0

### Styling

- Cargo fmt y fix clippy
## [0.8.0] - 2026-07-13

### Features

- Add search filter and column sorting to dashboard
- Merge feature/search-sort-dashboard into develop

### Miscellaneous Tasks

- Bump version to 0.8.0
- Update lockfile and vampus config for 0.8.0

### Other

- Merge v0.7.0 into develop
## [0.7.0] - 2026-07-13

### Bug Fixes

- Merge auto-update-pull-check into develop
- Gf-publish and gf-hotfix-publish use {{var}} instead of

### Features

- Remove redundant Stack column from dashboard table
- Merge feature/remove-stack-column into develop

### Miscellaneous Tasks

- Bump version to 0.7.0

### Other

- Align main with develop (cargo fmt)
- V0.7.0

### Styling

- Cargo fmt
## [0.6.1] - 2026-07-13

### Bug Fixes

- Unificar autenticación frontend a cookies de sesión OIDC
- Auto-update worker - check digest before pulling images
- Auto-update worker - check digest before pulling images
## [0.6.0] - 2026-07-12

### Other

- V0.6.0
## [0.5.2] - 2026-07-12

### Bug Fixes

- Has_update nunca se limpia tras check/update
- Merge fix-update-icon-persistence into develop
- Mostrar siempre nombre de imagen en lugar de hash sha256
- Recuperar nombre de imagen via inspect_container cuando Docker devuelve sha256
- Fix-update-icon-persistence

### Documentation

- Actualizar todo.md con el estado real del proyecto

### Features

- Tareas programadas con soporte para stacks y notificaciones
- Fase 2 - rollback en tareas programadas

### Miscellaneous Tasks

- Bump version to 0.6.0
- Fase 1 limpieza - eliminar terminal.rs, mover health_h a main.rs
- Fase 2 limpieza - mover config/history handlers a sus módulos
- Fase 3 limpieza - extraer persistence.rs de workers.rs

### Other

- Merge v0.6.0 into develop
## [0.5.1] - 2026-07-12

### Bug Fixes

- Detección precisa de actualizaciones + iconos PWA

### Miscellaneous Tasks

- Bump version to 0.5.1
- Regenerate Cargo.lock for v0.5.1

### Other

- Merge v0.5.0 into develop
- Merge develop into main
- Merge v0.5.1 into develop
- V0.5.1
## [0.5.0] - 2026-07-11

### Bug Fixes

- Remove unused config state variable from ConfigPage
- Show user name instead of UUID in header

### Features

- Add Settings types and FILE_SETTINGS constant
- Add Settings to AppState with JSON persistence
- Add PUT /api/config endpoint with settings-aware handler
- Workers and notifications are settings-aware
- Rewrite ConfigPage with Telegram, Matrix and Auto-update controls

### Miscellaneous Tasks

- Bump version to 0.5.0

### Other

- Merge v0.4.0 into develop
- Merge develop into main
- V0.5.0
## [0.4.0] - 2026-07-11

### Features

- Improve dashboard update flow and container menus
- Merge feature/smart-update into develop

### Miscellaneous Tasks

- Bump version to 0.4.0

### Other

- Merge v0.3.0 into develop
- Merge develop into main
- V0.4.0
## [0.3.0] - 2026-07-11

### Features

- Responsive UI with mobile card views
- Merge feature/responsive-ui into develop

### Miscellaneous Tasks

- Bump version to 0.3.0

### Other

- Merge v0.2.0 into develop
- V0.3.0
## [0.2.0] - 2026-07-11

### Bug Fixes

- Update jsonwebtoken 9 -> 10 with rust_crypto + commit Cargo.lock
- Use standard OIDC state parameter instead of custom state_id

### Documentation

- Add GIT_FLOW.md and gitflow recipes to justfile

### Features

- Merge feature/oidc-only-auth into develop
- Massive cleanup and simplification
- Merge cleanup-and-simplify into develop

### Miscellaneous Tasks

- Add alloy.container.example and stop tracking alloy.container with secrets

### Other

- V0.2.0
- V0.2.0
## [0.1.1] - 2026-07-11

### Bug Fixes

- Update jsonwebtoken 9 -> 10 with rust_crypto + commit Cargo.lock
## [0.1.0] - 2026-07-11

### Features

- Implement PocketID OIDC as the only authentication method
- Implement PocketID OIDC as the only authentication method

### Miscellaneous Tasks

- Initial project setup with .gitignore
