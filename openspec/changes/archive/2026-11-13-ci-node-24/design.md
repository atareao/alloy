# Design

## Context

- Tres workflows: `ci.yml` (PRs a `main`/`develop`), `release.yml` (push de tag `v*` +
  `workflow_dispatch`) y `release-prepare.yml` (push a `main` + `workflow_dispatch`).
- `Dockerfile` (multi-stage): `backend-builder` (Rust) · `frontend-builder` (`node:23-alpine`) ·
  runtime (`alpine:3.23`).
- `ci.yml` configura Node con `node-version: 23` y pnpm sin versión (de `packageManager`).
- Los runners ya ejecutan las actions sobre Node 24, pero las versiones actuales **declaran**
  Node 20 → warning en cada run.
- Solo `ci.yml` se ejecuta en un PR; los otros dos, no.

## Goals / Non-Goals

**Goals:**

- Eliminar los warnings de deprecación de Node 20 subiendo las actions a versiones con Node 24.
- Unificar la versión de Node en la LTS vigente (**24**) con una fuente única de verdad y una
  comprobación que impida la deriva.

**Non-Goals:**

- No se cambia la versión de pnpm (sigue en `packageManager`) ni la del `Dockerfile` se alinea con
  ella aquí.
- No se cambia la lógica de los workflows (bump con vampus, changelog con git-cliff, sync de
  `develop`, publicación).
- No se migran las actions *composite* ni se adoptan features nuevas de majors superiores.

## Decisions

### D1 — Versión mayor **mínima** que corre en Node 24

Para cada action se elige la **primera mayor** cuyo `action.yml` declara `runs.using: node24`,
no la última disponible. Motivo: `release.yml` y `release-prepare.yml` **no se pueden probar en
un PR**, así que se minimiza la superficie de cambios de comportamiento.

| Action | Actual | Node | Nueva | Node |
|---|---|---|---|---|
| `actions/checkout` | v4 | 20 | **v5** | 24 |
| `actions/upload-artifact` | v4 | 20 | **v6** | 24 |
| `actions/download-artifact` | v4 | 20 | **v7** | 24 |
| `actions/setup-node` | v4 | 20 | **v5** | 24 |
| `pnpm/action-setup` | v4 | 20 | **v5** | 24 |
| `docker/login-action` | v3 | 20 | **v4** | 24 |
| `docker/build-push-action` | v6 | 20 | **v7** | 24 |
| `softprops/action-gh-release` | v2 | (20) | **v3** | composite |

*Alternativa considerada*: saltar a la última mayor de cada una (checkout v7, download-artifact
v8, …). Rechazada: más cambios de comportamiento en un pipeline sin cobertura de PR, sin
beneficio inmediato.

### D2 — Fuente única de verdad: `engines.node` en `frontend/package.json`

`engines.node` (`^24`) es la declaración canónica. `ci.yml` la consume con
`node-version-file: frontend/package.json` (setup-node lee `engines.node`), y así no se declara
`node-version` por duplicado. Es el mismo principio que ya se aplica a pnpm (`packageManager`).

*Alternativa considerada*: `.nvmrc` como fuente. Rechazada: sería una cuarta ubicación y no
aporta nada frente a `engines`, que además documenta la compatibilidad.

### D3 — El `Dockerfile` fija el tag (`node:24-alpine`) y CI lo comprueba

Un `FROM` de Docker **no puede** leer `package.json`, así que el tag del `frontend-builder` es
necesariamente una declaración aparte. Para que la "fuente única" sea real y no un pacto verbal,
CI incluye un **guard** (D4).

### D4 — Guard de consistencia en `ci.yml`

Paso previo al setup de Node (shell puro, sin depender de Node instalado): extrae el major de
`engines.node` de `frontend/package.json` y el del `FROM …node:<major>-…` del `Dockerfile` y
falla si difieren. Convierte el requisito *Node alineado con el Dockerfile* en algo verificable.

*Alternativa considerada*: confiar en la revisión y la documentación. Rechazada: ya ha derivado
una vez (23 vs 24), un guard barato lo evita.

### D5 — Las actions *composite* no se tocan

`taiki-e/install-action`, `swatinem/rust-cache` y `dtolnay/rust-toolchain` son *composite*: no
declaran runtime Node, así que no entran en la deprecación. Se dejan en su major actual.

### D6 — Validación de los workflows no cubiertos por PR

`ci.yml` se valida en el propio PR. Para `release.yml`/`release-prepare.yml`: `actionlint`
(comprueba refs e inputs) en local + un `workflow_dispatch` de `release.yml` **tras mergear**,
verificando que los jobs `build`/`release`/`docker` siguen verdes y sin warnings de Node 20.

## Risks / Trade-offs

- **Majors nuevos en actions de release**: mitigado con D1 (mínima) + `actionlint` + dispatch de
  prueba. Si el dispatch falla, se revierte el ref concreto.
- **`engines.node` restrictivo** (`^24`): bloquea Node 25. Es intencional (queremos el major 24
  alineado con el `Dockerfile`); al subir de LTS se actualizan ambos.
- **El guard añade un paso a `ci.yml`**: coste mínimo (una línea de shell) frente a la garantía
  de no volver a divergir.
