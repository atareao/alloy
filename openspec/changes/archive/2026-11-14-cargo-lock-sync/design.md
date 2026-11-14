# Design

## Contexto

Drift detectado (commiteado, no local):

```
backend/Cargo.toml  →  version = "0.24.1"
backend/Cargo.lock  →  [[package]] name = "alloy" version = "0.23.8"
```

Origen: `.vampus.yml` solo reemplaza la versión en `backend/Cargo.toml`, así que el paso de bump de
`release-prepare.yml` deja el lock atrás y cada release re-introduce el drift. En local, `cargo`
(sin `--locked`) reescribe el lock silenciosamente al primer comando, ensuciando el árbol.

## Decisiones

### D1 — `cargo update -w` para sincronizar (no `cargo update`)

`cargo update -w` (azúcar de `cargo update --workspace`, disponible en cargo ≥ 1.79; aquí 1.97)
**solo** actualiza la versión del propio paquete en el lock (`backend` es un paquete único, sin
`[workspace]` ni `Cargo.toml` raíz; `-w` funciona igual). Verificado:

```
$ cargo update -w
     Locking 1 package to latest compatible version
    Updating alloy v0.23.8 (/data/rust/alloy/backend) -> v0.24.1
```

Cambia 1 línea del lock. Frente a `cargo update` a secas (que actualiza *todas* las dependencias a
su última versión compatible, sin revisión), es determinista y respeta el "sin cambios de deps".

### D2 — `--locked` en CI en vez de un paso ad-hoc

`--locked` es la bandera estándar de cargo: hace fallar el comando si `Cargo.lock` tendría que
cambiar. Se añade a los tres comandos del job `test` que resuelven dependencias:
`cargo test`, el listado `-- --ignored --list` y `cargo clippy`. También a la build del artefacto en
`release.yml` (`cargo build --release`), para que un lock stale haga *fallar* el release en lugar de
autocorregirse. `cargo fmt` no resuelve dependencias y no admite/necesita `--locked`.

Alternativa descartada: un paso `cargo metadata --locked` dedicado. Repetiría la misma comprobación
con un comando extra; `--locked` en los comandos existentes da el mismo fallo con menos superficie.

### D3 — El release sincroniza el lock

`release-prepare.yml`, tras `vampus upgrade`, ejecuta
`cargo update -w --manifest-path backend/Cargo.toml` y lo incluye en el `git add -A` del commit
`chore: release vX`. Sin esto, la guarda de `--locked` rompería todos los PRs posteriores al primer
release (el lock quedaría monoversión por detrás).

### D4 — `just check` y `just upgrade`

- `just check` (pre-commit del repo) pasa `clippy` a `--locked`: el drift se detecta antes de
  commitear, no solo en CI. `just lint` y `gf-publish` también, por coherencia.
- `just upgrade` (receta legacy manual) cambia `cargo update` → `cargo update -w`; así no arrastra
  actualizaciones de dependencias sin revisar.

No se añade `--locked` a `just test` para no impedir el trabajo local normal (equivale a tener que
sincronizar el lock para poder pasar tests); el objetivo es impedir *commitear* el drift, que ya
cubren `just check` y CI.

### D5 — El delta de `ci/release-automation`

Como `release-prepare.yml` está gobernado por la capability `ci/release-automation`, el requisito de
bump con vampus se modifica allí (delta `MODIFIED`) para incluir la sincronización del lock. El
comportamiento nuevo queda así trazado en la capability que le corresponde, y `ci/rust-lockfile` cubre
el invariante del lock y su verificación (CI, release build, guard local).

## Riesgos

- **Falso fallo en CI por caché**: `--locked` no descarga ni actualiza; si el lock está sincronizado,
  no cambia nada. Riesgo nulo.
- **`cargo update -w` sin red**: para cambiar solo la versión del miembro no necesita resolver la
  red de dependencias, pero cargo puede querer el índice; en CI el paso `cargo install vampus` ya lo
  ha descargado.
- `just upgrade` es una receta legacy; puede quedar obsoleta frente a `release-prepare.yml`. Se
  ajusta por coherencia, no se elimina (fuera de alcance).
