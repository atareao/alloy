# GIT_FLOW.md — Flujo de trabajo GitFlow para alloy

## Estructura de ramas

```
main        → Producción. Solo recibe merges de release/* y hotfix/*
develop     → Integración. Recibe merges de feature/*
feature/*   → Nuevas funcionalidades (desde develop)
release/*   → Preparación de release (desde develop)
hotfix/*    → Arreglos urgentes en producción (desde main)
```

## Convenciones de nomenclatura

| Tipo | Formato | Ejemplo |
|------|---------|---------|
| Feature | `feature/<nombre-corto>` | `feature/dark-mode` |
| Release | `release/<version>` | `release/0.2.0` |
| Hotfix | `hotfix/<descripcion>` | `hotfix/critical-auth-bug` |

## Flujo de trabajo

### 1. Nueva funcionalidad (feature)

```sh
# Desde develop
git checkout develop
git checkout -b feature/mi-feature

# Trabajar, commitear, pushear...
git add .
git commit -m "feat: descripción del cambio"
git push -u origin feature/mi-feature

# Al terminar: merge a develop
git checkout develop
git merge --no-ff feature/mi-feature -m "feat: merge feature/mi-feature into develop"
git push origin develop
git branch -d feature/mi-feature
```

### 2. Preparar un release

```sh
# Desde develop, cuando está listo para producción
git checkout develop
git checkout -b release/0.2.0

# Últimos retoques (version bump, docs, etc.)
# Los fixes van aquí, NO en develop

# Al terminar: merge a main + develop
git checkout main
git merge --no-ff release/0.2.0 -m "release: v0.2.0"
git tag -a v0.2.0 -m "Version 0.2.0"

git checkout develop
git merge --no-ff release/0.2.0 -m "release: merge v0.2.0 into develop"
git branch -d release/0.2.0

git push origin main --tags
git push origin develop
```

### 3. Hotfix (arreglo urgente en producción)

```sh
# Desde main
git checkout main
git checkout -b hotfix/descripcion

# Arreglar, commitear
git commit -m "fix: descripción del arreglo"

# Merge a main + develop
git checkout main
git merge --no-ff hotfix/descripcion -m "hotfix: descripción"
git tag -a v0.2.1 -m "Hotfix v0.2.1"

git checkout develop
git merge --no-ff hotfix/descripcion -m "hotfix: merge into develop"
git branch -d hotfix/descripcion

git push origin main --tags
git push origin develop
```

## Pre-commit checklist (Rust)

Antes de cualquier commit en ramas feature/*:

```sh
cd backend && cargo fmt -- --check
cd backend && cargo clippy -- -D warnings
```

No commitear si cualquiera de los dos falla.

## Commits semánticos

| Tipo | Para |
|------|------|
| `feat:` | Nueva funcionalidad |
| `fix:` | Corrección de bug |
| `chore:` | Tareas de mantenimiento (build, config, CI) |
| `docs:` | Cambios en documentación |
| `refactor:` | Refactorización sin cambios funcionales |
| `test:` | Añadir o modificar tests |
| `style:` | Formato, linting, whitespace |

## Resumen visual

```
main:      o----------------o----------------o
           \              / \              /
            \            /   \            /
release/*    \          /     \          /
              \        /       \        /
develop:       o------o---------o------o
               |\    /           \    /|
               | \  /             \  / |
feature/*      |  o                o   |
               |  |                |   |
               o--+----------------+---o
```

## Atajos con just

```sh
just gf-feature  nombre    # Iniciar una feature
just gf-finish   nombre    # Finalizar y mergear feature a develop
just gf-release  version   # Iniciar un release
just gf-hotfix   desc      # Iniciar un hotfix
just gf-publish  version   # Finalizar release: merge a main + tag + push
```

Ver `just --list` para más detalles.