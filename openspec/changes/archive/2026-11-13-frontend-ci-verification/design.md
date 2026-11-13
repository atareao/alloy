# Design

## Context

`.github/workflows/ci.yml` tiene un único job `test` que ejecuta, sobre `backend/`:
`cargo test --manifest-path backend/Cargo.toml`, `cargo fmt -- --check` y
`cargo clippy -- -D warnings`. No hay ningún paso de Node ni de frontend.

Hechos relevantes:

- El frontend tiene **72 tests** en 8 ficheros (`vitest run`), type-check con `tsc -b`, build con
  Vite y lint con oxlint. Todo pasa hoy en verde, verificado localmente.
- `frontend/package.json` declara `packageManager: "pnpm@11.21.0"`, y `frontend/pnpm-lock.yaml`
  es su único lockfile.
- El `Dockerfile` construye el frontend sobre `node:23-alpine`.
- El entorno de desarrollo local usa Node `v24.22.1`: ya existe divergencia con el Dockerfile.
  No hay `.nvmrc` ni campo `engines`.
- El job de backend pasa en CI porque los tests de integración de `containers`, que dependen de
  Podman, **se auto-omiten** cuando no hay socket (`is_podman_available()`): en `ubuntu-latest`
  no existe `/run/user/1000/podman/podman.sock`. Es decir, la independencia de jobs no es un
  detalle teórico.

## Goals / Non-Goals

**Goals:**

- Que un PR con el frontend roto **no pueda** quedar en verde.
- Que la verificación use la misma toolchain que el build de producción (pnpm y Node).
- Que el fallo sea atribuible: un paso por comprobación, con nombre explícito.

**Non-Goals:**

- **No** se toca el job de backend ni su `--manifest-path`.
- **No** se añade `pnpm audit` a CI: depende de la base de avisos en red y bloquearía PRs por un
  advisory nuevo ajeno al cambio. La higiene de dependencias se cubre con Dependabot y con el
  spec `security/dependency-hygiene`.
- **No** se sube la versión de Node del `Dockerfile`. Node 23 está fuera de soporte (fue una
  release non-LTS), pero cambiarlo altera el build de producción y es otro alcance: se registra
  como follow-up junto con la unificación de la versión en local (`24.22.1`) y CI.
- **No** se filtran los jobs con `paths:`. Se prefiere un check determinista y siempre presente
  antes que ahorrar segundos a costa de que un PR se salte la verificación.
- **No** se tocan `release.yml` ni `release-prepare.yml`.

## Decisions

### D1 — Job separado `frontend`, no pasos añadidos al job `test`

Se añade un job nuevo en paralelo. Alternativa descartada: meter los pasos del frontend al final
del job `test`. Motivos: (a) los fallos quedan atribuidos a un job con nombre, no mezclados con
los del backend; (b) ambos corren en paralelo, así que el tiempo total no se suma; (c) un fallo
del backend no impide ver el resultado del frontend y viceversa (requirement de independencia).

### D2 — `pnpm/action-setup@v4` sin `version`, leyendo `packageManager`

El workflow **no** declara la versión de pnpm. Se usa `pnpm/action-setup@v4` sin `version`, que
la resuelve desde el campo `packageManager` que la acción lee de `frontend/package.json`. Así el
pin del gestor vive en un único sitio y no puede divergir del que usa el `Dockerfile`.

*Alternativa descartada:* `corepack enable`. Funciona, pero obliga a invocar `actions/setup-node`
dos veces para poder usar su `cache: pnpm` (el cacheado necesita pnpm ya en el `PATH`), lo que
complica el workflow sin ganar nada.

*Alternativa descartada:* `pnpm/action-setup@v4` con `version: 11.21.0` explícito. Duplica el pin
y reintroduce exactamente el problema que el change anterior eliminó.

### D3 — Node 23, alineado con el `Dockerfile`

Se fija `node-version: 23` porque es la versión sobre la que se construye el artefacto de
producción. La regla que aplica este change es *paridad de toolchain*, no *modernidad del
runtime*: si CI usa 24 y producción 23, la verificación deja de ser representativa del artefacto
que se despliega.

Esto deja a la vista que Node 23 está EOL y que local ya va por 24. Es un problema real, pero
resolverlo significa **decidir** qué versión soporta Alloy y cambiarla en tres sitios (Dockerfile,
CI y, opcionalmente, un campo `engines`/`.nvmrc` como fuente única). Mezclarlo aquí haría que este
change no fuera verificable por separado. Queda como follow-up explícito.

### D4 — Un paso → una comprobación

`tsc -b`, `vitest run`, `vite build` y `oxlint` van en pasos separados con nombre propio, aunque
`pnpm run build` ya encadena `tsc -b && vite build`. Se prefiere el paso redundante a un
`failure` que no diga si el problema son los tipos o el bundler.

### D5 — La "fase RED" es de verificación en CI real

El cambio no introduce lógica de aplicación, así que no hay tests nuevos que escribir. La
evidencia de RED es el estado actual (el workflow no menciona Node ni el frontend) y la de GREEN
es la ejecución real del workflow: el PR que introduce el job debe mostrar el job `frontend` en
verde. Además se comprueba que **falla de verdad**, empujando temporalmente un error de tipos.

## Risks / Trade-offs

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Node 23 EOL en CI | Medio — sin parches de seguridad en el runner | Follow-up: unificar Dockerfile, CI y local en la LTS vigente con una sola fuente de verdad. |
| El job destapa fallos latentes | Bajo | Los 72 tests, `tsc`, el build y el lint pasan hoy verificados a mano; si algo aparece, es información real, no ruido. |
| Warnings ruidosos de jsdom (`Window's getComputedStyle()`) | Bajo | Ya aparecen en local sin romper la suite; no convierten el job en rojo. |
| Coste de CI (~1 min/PR) | Bajo | Job paralelo; no alarga la ruta crítica. |
| Divergencia Node local (24) vs CI/Docker (23) | Bajo | No afecta a esta verificación (todo pasa en 24); se registra en el follow-up de unificación. |
