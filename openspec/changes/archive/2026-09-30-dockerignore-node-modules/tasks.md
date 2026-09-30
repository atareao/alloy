# Tasks

## 1. Implementación

- [x] 1.1 Añadir `frontend/node_modules/` a `.dockerignore`
- [x] 1.2 Añadir `ENV CI=true` al stage `frontend-builder` del `Dockerfile`

## 2. Verificación

- [x] 2.1 `podman build -t alloy-test .` completa sin `ERR_PNPM_ABORTED_REMOVE_MODULES_DIR_NO_TTY`
- [x] 2.2 Imagen construida y taggeada correctamente (52.8 MB); imagen de prueba eliminada

## 3. Archive

- [ ] 3.1 `openspec validate dockerignore-node-modules --strict`
- [ ] 3.2 `openspec archive dockerignore-node-modules --yes`
- [ ] 3.3 Verificar/eliminar `openspec/changes/dockerignore-node-modules/`
