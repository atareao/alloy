# Deduplicar `@ant-design/cssinjs`

Issue: #190 (follow-up de #163 fase 1). Change OpenSpec: `openspec/changes/dedupe-antd-cssinjs/`.
Plan: `plans/PLAN-028.md`.

## Qué es

El lock del frontend resolvía **dos copias** del runtime de estilos de antd:

| Versión | Quién la pedía | Rango declarado |
|---|---|---|
| `2.1.2` | `antd@6.7.1` | `~2.1.2` |
| `2.2.0` | `@ant-design/cssinjs-utils@2.1.2` | `^2.1.2` |

Dos copias del mismo runtime duplican código en el bundle y pueden crear dos
cachés de estilos distintas (`StyleProvider`/`useStyleRegister` con instancias
diferentes). Tras el cambio, el árbol resuelve **una sola** copia: `2.1.2`.

## Ficheros implicados

- `frontend/pnpm-workspace.yaml` (**nuevo**) — `overrides` con `@ant-design/cssinjs: 2.1.2`.
- `frontend/pnpm-lock.yaml` — regenerado (`pnpm install --no-frozen-lockfile`).
- `docs/frontend-deps.md` — nota actualizada (antes decía que no se forzaba).

`frontend/package.json` **no** cambia: en pnpm 11 los ajustes ya no se leen del
campo `pnpm` de `package.json` (se ignoran con warning). El sitio correcto es
`pnpm-workspace.yaml`.

## Decisión: por qué `2.1.2` y no `2.2.0`

- `2.1.2` satisface **los dos** rangos: `~2.1.2` (antd) y `^2.1.2` (cssinjs-utils).
- `2.2.0` queda **fuera** de `~2.1.2`, que es el rango que declara antd 6.7.1:
  forzarlo sería empujar a antd una versión que su propio `package.json` excluye.
- `2.1.2` es además la versión que antd ya usa, así que es la que está probada
  contra los estilos de antd 6.7.1.

**Plan B documentado:** si el build o los tests hubieran fallado con `2.1.2`
(p. ej. porque `cssinjs-utils` usara una API introducida en `2.2.0`), la
alternativa era fijar `2.2.0` y revisar que antd no dependa de APIs de `2.1.x`.
No ha sido necesario: `2.1.2` pasa todas las verificaciones.

## Cómo funciona

`frontend/pnpm-workspace.yaml`:

```yaml
overrides:
  "@ant-design/cssinjs": "2.1.2"
```

pnpm aplica el override a toda resolución del paquete en el árbol, así que
`@ant-design/cssinjs-utils` recibe `2.1.2` aunque declare `^2.1.2`.

## Verificación reproducible

Desde `frontend/`:

```bash
# 1. Una sola versión en el árbol
pnpm why @ant-design/cssinjs
# -> "Found 1 version of @ant-design/cssinjs"

# 2. Una sola versión distinta en el lock (cuenta versiones, no líneas)
rg -o "@ant-design/cssinjs@[0-9]+\.[0-9]+\.[0-9]+" pnpm-lock.yaml | sort -u
# -> una sola línea: @ant-design/cssinjs@2.1.2

# 3. Sin rastro de 2.2.0
rg "cssinjs@2\.2\.0" pnpm-lock.yaml
# -> sin resultados (exit 1)

# 4. Calidad
pnpm exec tsc -b
pnpm run lint
npx vitest run
pnpm run build
```

> Ojo al interpretar el lock: una misma versión aparece dos veces con el
> prefijo `@ant-design/cssinjs@` — la entrada en `packages:` y su resolución
> con peers en `snapshots:`. Por eso `rg -c "@ant-design/cssinjs@" pnpm-lock.yaml`
> devuelve **2** aun estando deduplicado; el criterio válido es el número de
> **versiones distintas** (`rg -o … | sort -u`), que es 1.

> Tras cambiar `overrides`, conviene reinstalar en limpio
> (`rm -rf node_modules && pnpm install --frozen-lockfile`): `.pnpm/` puede
> conservar copias residuales de la versión retirada aunque el lock ya esté bien.

### Estado de partida (RED)

```
$ pnpm why @ant-design/cssinjs
@ant-design/cssinjs@2.1.2
└─┬ antd@6.7.1
  └── frontend@0.0.0 (dependencies)

@ant-design/cssinjs@2.2.0
└─┬ @ant-design/cssinjs-utils@2.1.2
  └─┬ antd@6.7.1
    └── frontend@0.0.0 (dependencies)

Found 2 versions of @ant-design/cssinjs
```

En el lock había 4 líneas con el prefijo `@ant-design/cssinjs@`: dos entradas
en `packages:` (`'@ant-design/cssinjs@2.1.2':` y `'@ant-design/cssinjs@2.2.0':`)
y sus dos resoluciones en `snapshots:` (`...@2.1.2(react-dom…)` y
`...@2.2.0(react-dom…)`).

### Estado final (GREEN)

```
$ pnpm why @ant-design/cssinjs
@ant-design/cssinjs@2.1.2
├─┬ @ant-design/cssinjs-utils@2.1.2
│ └─┬ antd@6.7.1
│   └── frontend@0.0.0 (dependencies)
└── antd@6.7.1 [deduped]

Found 1 version of @ant-design/cssinjs
```

En el lock quedan 2 líneas con el prefijo, ambas de `2.1.2`: la entrada en
`packages:` (`'@ant-design/cssinjs@2.1.2':`) y su resolución en `snapshots:`
(`'@ant-design/cssinjs@2.1.2(react-dom@19.3.1(react@19.3.1))(react@19.3.1)'`).
Sin bloque `2.2.0`.

### Resultados

| Comando | Resultado |
|---|---|
| `pnpm exec tsc -b` | 0 |
| `pnpm run lint` (`oxlint --deny-warnings`) | 0, sin warnings |
| `npx vitest run` | 209 tests / 19 ficheros, 0 |
| `pnpm run build` | 0 |

Bundle antes/después (una sola copia menos del runtime de estilos):

| Métrica | Antes | Después |
|---|---|---|
| Módulos transformados | 3157 | 3125 (−32) |
| JS minificado | 1.152,95 kB | 1.145,33 kB (−7,62 kB) |
| JS gzip | 356,90 kB | 356,18 kB (−0,72 kB) |

## Follow-ups

- **Imagen Docker:** el `Dockerfile` debe copiar `frontend/pnpm-workspace.yaml`
  al contexto de build; si no, la instalación dentro de la imagen ignorará el
  override y volverá a resolver dos copias.
- **Revisar cuando antd suba su rango** de `@ant-design/cssinjs` (p. ej. a
  `~2.2.x` o `^2.2`). En ese momento el override a `2.1.2` pasará de
  deduplicador a pin innecesario: subirlo a la versión que antd pida o
  eliminarlo si ya no hay duplicado.
- Vigilar otros duplicados del lock; este change solo toca `cssinjs`.
- `pnpm-lock.yaml` se regeneró sin `--frozen-lockfile`; el CI sigue usando
  `--frozen-lockfile` y debe pasar con el lock actual.