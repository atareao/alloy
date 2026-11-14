# Design

## Contexto

```yaml
matrix:
  include:
    - target: x86_64-unknown-linux-gnu
      archive: tar.gz
    - target: aarch64-unknown-linux-gnu
      archive: tar.gz
...
if [ "${{ matrix.archive }}" = "tar.gz" ]; then
  tar czf "$PKG.tar.gz" "$ARCHIVE_NAME"
  echo "ARTIFACT=$PKG.tar.gz" >> "$GITHUB_ENV"
else
  7z a "$PKG.zip" "$ARCHIVE_NAME"          # inalcanzable: ningún target usa zip
  echo "ARTIFACT=$PKG.zip" >> "$GITHUB_ENV"
fi
```

## Decisiones

### D1 — Eliminar la rama, no arreglarla

Se podría sustituir `7z` por `zip` (disponible en los runners) para "revivir" la rama, pero seguiría
siendo inalcanzable mientras la matriz no declare un `archive: zip`. La corrección correcta es
**eliminar el código muerto**, no mantener una rama que nadie ejecuta.

### D2 — Quitar la clave `archive` de la matriz

`matrix.archive` solo existía para alimentar el `if`. Sin el `if` (y sin ningún target zip), la clave
queda sin consumidor. Se elimina para no dejar configuración muerta. Se conserva `ext` (participa en
`alloy${{ matrix.ext }}` y documenta el sufijo por target, hoy vacío en Linux).

### D3 — Limpiar las referencias colgantes del job `release`

Como no se produce ningún `.zip`, se retiran `*.zip` de `files:` y `s/\.zip$//` del `sed`. Así la
tabla de descargas y los adjuntos corresponden 1:1 a lo que produce el build.

## Fuera de alcance

- Añadir un target `zip`/Windows (si algún día se quiere, entonces sí habría que reintroducir el
  formato y verificar la herramienta de compresión).
