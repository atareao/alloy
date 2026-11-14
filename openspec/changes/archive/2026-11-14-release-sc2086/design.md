# Design

## Contexto

```
$ actionlint .github/workflows/*.yml
release.yml:37:9: SC2086:info:7:34:  Double quote to prevent globbing and word splitting
release.yml:37:9: SC2086:info:10:31: Double quote to prevent globbing and word splitting
```

Ambos apuntan a `echo "ARTIFACT=…" >> $GITHUB_ENV` (líneas 44 y 47). `actionlint` no estaba
conectado a CI, así que el aviso solo se veía ejecutándolo a mano.

## Decisiones

### D1 — Comillar `"$GITHUB_ENV"`

`$GITHUB_ENV` es una ruta absoluta a un fichero temporal; en la práctica no contiene espacios, así
que el aviso es de higiene, no de corrección. La corrección canónica de `SC2086` es comillar la
variable: `>> "$GITHUB_ENV"`. Sin efecto sobre el valor exportado.

### D2 — Job de CI con la imagen de `actionlint` fijada

Nuevo job `workflows` en `ci.yml`:

```yaml
  workflows:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@v5
      - name: Lint GitHub Actions workflows
        run: docker run --rm --network none -v "$PWD:/mnt:ro" -w /mnt rhysd/actionlint:1.7.12
```

- Se ejecuta **sin argumentos**: `actionlint` autodescubre `.github/workflows/`. Pasar
  `.github/workflows/` como argumento **falla** (`… is a directory`, exit 3).
- `permissions: contents: read` (mínimo): el job solo lee el repo.
- Montaje `:ro` y `--network none`: `actionlint` solo lee; el contenedor no necesita red ni escritura.
- `actions/checkout` es imprescindible: `actionlint` exige estar dentro de un repo git.
- La versión de la imagen se fija a `1.7.12` (la misma que valida el repo en local) para que el
  resultado sea reproducible. Se prefiere la imagen al script oficial de descarga (`bash <(curl …)`)
  porque este ejecuta código remoto sin fijar; en un repo con `docs/security-dependencies.md` que
  cuida la cadena de suministro, la imagen fijada por tag es preferible (coherente con las actions
  del repo, fijadas por major tag).

### D3 — `shellcheck` es imprescindible y lo aporta la imagen

`actionlint` **no** detecta `SC####` por sí solo: delega en `shellcheck`. Sin `shellcheck` en `PATH`,
`actionlint` desactiva esa integración y **sale 0 aunque el bug esté presente** (verificado:
`actionlint -shellcheck= …` → exit 0 con `>> $GITHUB_ENV` sin comillar). Por eso el guard usa la
imagen `rhysd/actionlint`, que **incluye `shellcheck`** (verificado: reporta `SC2086` en un workflow
de prueba). Queda documentado para que nadie "simplifique" el job a un `go install` sin `shellcheck`.

### D4 — Alcance y trade-offs

- El trigger de `ci.yml` es `pull_request` sobre `main` y `develop`: el guard cubre los PRs.
  `release-prepare.yml` empuja commits de bot directamente a `main`/`develop` (con `GH_PAT`) y no pasa
  por el guard; riesgo bajo (contenido generado).
- El `docker run` hace un pull anónimo de Docker Hub en cada run. Se acepta: el pin garantiza
  reproducibilidad y el repo no tiene credenciales de Docker Hub. Si aparecieran flake por rate limit,
  la mitigación es fijar la imagen por **digest**.
- No se añade receta `just` de lint de workflows (el guard vive en CI, que es donde aplica a los PRs).

## Fuera de alcance

- La rama `else` de *Prepare archive* (`7z`, `ARTIFACT=$PKG.zip`) es **código muerto**: ambas entradas
  de la matriz usan `archive: tar.gz`. El fix la toca de pasada, pero no se ejercita.
