# Sincronización de `backend/Cargo.lock`

Cómo se evita que el lockfile de Rust se desincronice de `backend/Cargo.toml` y cómo se detecta.

## Síntoma

El `backend/Cargo.lock` **commiteado** puede llevar la versión raíz desincronizada:

```
backend/Cargo.toml  →  version = "0.24.1"
backend/Cargo.lock  →  [[package]] name = "alloy" version = "0.23.8"   ← stale
```

Efecto: cualquier `cargo` local **sin** `--locked` reescribe el lock al primer comando, así que el
árbol queda sucio (`M backend/Cargo.lock`) en cuanto se toca cualquier cosa, y es fácil commitear un
lock a medias o arrastrar cambios de dependencias sin revisar.

## Origen

`.vampus.yml` solo sustituye la versión en el manifiesto:

```yaml
replaces:
- file: backend/Cargo.toml
  pattern: ^version\s*=\s*"{{current_version}}"$
```

`release-prepare.yml` (bump con vampus) actualizaba `Cargo.toml` pero **no** el lock, así que cada
release re-introducía el drift.

## El contrato

### 1. El lock commiteado está sincronizado

`backend/Cargo.lock` debe declarar la misma versión para el paquete raíz `alloy` que
`backend/Cargo.toml`. Para sincronizarlo **sin tocar dependencias**:

```bash
cd backend && cargo update -w
```

`cargo update -w` (azúcar de `cargo update --workspace`, cargo ≥ 1.79) actualiza **solo** la versión
del propio paquete en el lock (`backend` es un paquete único, sin `[workspace]`). Cambia una línea:

```
$ cargo update -w
     Locking 1 package to latest compatible version
    Updating alloy v0.23.8 (/data/rust/alloy/backend) -> v0.24.1
```

No usar `cargo update` a secas: subiría *todas* las dependencias a su última versión compatible sin
revisión.

### 2. CI bloquea el drift

El job `test` de `ci.yml` ejecuta los comandos que resuelven dependencias con `--locked`:

```yaml
- run: cargo test --manifest-path backend/Cargo.toml --locked
- run: LIST=$(cargo test --manifest-path backend/Cargo.toml --locked -- --ignored --list)
- run: cargo clippy --manifest-path backend/Cargo.toml --locked -- -D warnings
```

Con el lock desincronizado, `--locked` falla:

```
error: cannot update the lock file .../backend/Cargo.lock because --locked was passed to prevent this
```

La build del artefacto en `release.yml` también usa `cargo build --release --locked`, para que el
binario publicado se construya siempre desde el lock commiteado.

`cargo fmt` no resuelve dependencias y no acepta `--locked`.

### 3. El release mantiene el lock al día

`release-prepare.yml`, tras el bump de vampus, sincroniza el lock antes de commitear:

```yaml
- name: Sync Cargo.lock with the new version
  run: cargo update -w --manifest-path backend/Cargo.toml
```

Sin este paso, la guarda de `--locked` rompería todos los PRs posteriores al primer release.

### 4. Local: `just check` y `just upgrade`

- `just check` (pre-commit del repo) usa `cargo clippy --locked`: el drift se detecta antes de
  commitear, no solo en CI. `just lint` y `gf-publish` también llevan `--locked`.
- `just upgrade` (receta legacy manual) usa `cargo update -w` en vez de `cargo update`, para no
  arrastrar actualizaciones de dependencias sin revisar.

## Cómo verificar

```bash
# El lock está sincronizado
cargo metadata --manifest-path backend/Cargo.toml --locked --format-version 1 > /dev/null && echo OK

# Prueba de mutación: fuerza el drift en el lock y comprueba que --locked DEBE fallar
# (adapta la versión del `sed` a la real del lock)
sed -i 's/^version = "0\.24\.1"$/version = "0.0.0-mutation"/' backend/Cargo.lock
cargo metadata --manifest-path backend/Cargo.toml --locked   # → error: cannot update the lock file
cd backend && cargo update -w                                # revierte el lock a la versión real

# El diff del lock solo cambia la línea de versión raíz
git diff backend/Cargo.lock
```

## Limitaciones

- `--locked` no está en `just test` (impediría el flujo local normal); el objetivo es impedir
  **commitear** el drift, cubierto por `just check` y CI.
- `just upgrade` es una receta legacy que puede quedar obsoleta frente a `release-prepare.yml`; se
  mantiene coherente, no se elimina.
