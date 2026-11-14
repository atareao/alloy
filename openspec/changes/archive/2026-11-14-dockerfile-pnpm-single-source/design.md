# Design

## Contexto

```
Dockerfile:32            RUN npm install -g pnpm@11.21.0     ← pin duplicado
frontend/package.json:6  "packageManager": "pnpm@11.21.0"    ← fuente única (ya usada por CI)
```

El workflow de CI ya consume `packageManager` (`pnpm/action-setup@v5` con `package_json_file`, sin
`version`). El Dockerfile es el único consumidor que todavía fija la versión a mano.

## Decisiones

### D1 — Derivar la versión con `node -p` (no corepack)

`node:24-alpine` trae Node y npm, y el Dockerfile ya usa `npm install -g pnpm@…`. En vez de fijar la
versión:

```dockerfile
COPY frontend/package.json ./
RUN PNPM_VERSION=$(node -p 'require("./package.json").packageManager.replace(/^pnpm@/, "").split("+")[0]') \
    && case "$PNPM_VERSION" in ""|*[!0-9.]*) echo "packageManager debe declarar una version semver de pnpm; valor: '$PNPM_VERSION'" >&2; exit 1;; esac \
    && npm install -g "pnpm@$PNPM_VERSION"
```

Se **normaliza y valida** el valor antes de usarlo: se quita el prefijo `pnpm@` y cualquier sufijo de
integridad (`+sha512…`, que escribe `corepack`), y se comprueba que el resultado sea solo dígitos y
puntos. Así un `packageManager` con hash (`pnpm@11.21.0+sha512…` → `11.21.0`) o ajeno (`yarn@4.1.0`)
no produce una versión inexistente en el registry ni se reescribe en silencio como `pnpm@4.1.0`.

Ventajas frente a `corepack`:
- **Determinista y verificable**: el valor sale del propio manifiesto; no depende de que corepack
  esté presente ni de su política (corepack deja de distribuirse con Node ≥ 25).
- **Sin red extra**: reutiliza `npm`, ya presente.
- Con hash de integridad, `corepack` sería más correcto (verifica el hash); aquí se descarta ese
  sufijo explícitamente, que es el comportamiento documentado y suficiente para este repo.
- `node -p` lee el campo `packageManager`, así que la referencia a `packageManager` queda explícita
  en el Dockerfile (lo que aprovecha el guard de CI, D2).

Se adelanta el `COPY frontend/package.json ./` antes de instalar pnpm (hoy se copiaba junto al
lockfile). El lockfile se copia en su propio `COPY` justo antes de `pnpm install --frozen-lockfile`,
preservando el cacheo de capas.

Comillas: expresión con comillas simples dentro de `$(...)` (cuyo texto se parsea aparte), de modo
que no chocan con las comillas dobles de `"pnpm@$…"`.

### D2 — Guard de CI análogo al de Node

En `ci.yml`, tras `Check Node version consistency`, un paso `Check pnpm version consistency` que:

1. Lee `packageManager` de `frontend/package.json` (con `grep`, sin depender de que `node` esté
   instalado todavía en el job — `setup-node` corre después).
2. Descarta los comentarios del `Dockerfile` (`grep -vE '^[[:space:]]*#'`): un comentario **no**
   cuenta ni como deriva ni como hardcode (evita el falso negativo de "el comentario menciona
   `packageManager`").
3. **Falla** si el código del `Dockerfile` contiene el **valor literal** de la versión declarada
   (`grep -F "$PKG_PNPM_VER"`). Caza tanto `pnpm@11.21.0` como un `ARG PNPM_PIN=11.21.0` reutilizado
   en `pnpm@${PNPM_PIN}`.
4. **Falla** si el código del `Dockerfile` no referencia `packageManager` (regresión: alguien deja
   de derivar).

Alternativa descartada: mantener el pin y comprobar igualdad (como Node). No hace falta: si el
Dockerfile deriva, la duplicación desaparece de raíz. El guard solo protege contra reintroducirla.

### D3 — Documentación

`docs/pnpm-single-package-manager.md` (sección `packageManager` → pasa a ser la fuente única también
en el Dockerfile) y `docs/frontend-ci-verification.md` (tabla de coherencia de toolchain + mención
del guard). Se retira el follow-up ya resuelto de `docs/ci-node-24.md`.

## Riesgos

- **Build del stage `frontend-builder`**: el `COPY` adelantado no invalida el cacheo habitual
  (package.json cambia menos que el resto del frontend); el lockfile sigue cacheado en su propia capa.
- **`node -p` falla si desaparece `packageManager`**: es el comportamiento deseado — el build debe
  romper si la fuente única deja de existir.
- **Guard con `grep`**: en `ubuntu-latest` con `set -e` implícito en `run: |` de bash; los `grep`
  que deben poder no encontrar nada usan `if`/`|| true` para no abortar el paso por error.
