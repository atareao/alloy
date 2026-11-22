# Política de cero warnings de oxlint

## Qué es

El frontend exige **cero warnings**. El script de lint se ejecuta con
`--deny-warnings`, así que cualquier warning (no solo los errores) hace fallar
el comando con exit code distinto de 0. La misma política se aplica en local y
en CI: un warning nuevo rompe la build del PR.

## Dónde se configura

| Fichero | Contenido relevante |
|---|---|
| `frontend/package.json` | `"lint": "oxlint --deny-warnings"` y `oxlint` como dependencia de desarrollo |
| `frontend/.oxlintrc.json` | `plugins: ["react", "typescript", "oxc"]`, `react/rules-of-hooks: error`, `react/only-export-components: warn` (`allowConstantExport: true`) |

`.oxlintrc.json` activa 3 plugins además del conjunto base de eslint. El set
efectivo por defecto es la categoría `correctness`: 129 reglas (57 eslint +
31 react + 27 typescript + 14 oxc) más las 2 reglas declaradas explícitamente,
**131 reglas activas** en total con la configuración actual. Como
`only-export-components` está en `warn`, `--deny-warnings` hace que también
rompa el lint.

## Cómo se ejecuta

```bash
cd frontend
pnpm run lint          # equivale a: oxlint --deny-warnings
```

En CI, el job `frontend` de `.github/workflows/ci.yml` ejecuta
`pnpm run lint` (con `working-directory: frontend`) después de type check,
tests y build. Cualquier warning tumba el job.

## Qué hacer ante un warning nuevo

1. **Arreglar la causa.** Es la opción por defecto: ajustar el código para que
   la regla deje de dispararse.
2. **Solo si es un falso positivo justificado**, silenciar la regla en la línea
   concreta y documentar el motivo:

   ```tsx
   // oxlint-disable-next-line react/only-export-components -- el helper se exporta para tests
   export const helper = ...
   ```

3. **Nunca** bajar `--deny-warnings` del script ni desactivar reglas en
   `.oxlintrc.json` para "pasar" el lint. Eso rompe la garantía de la política.

## Verificación

La baseline debe dar **exit 0** y **salida vacía** (sin líneas de diagnóstico):

```bash
cd frontend && pnpm run lint; echo "EXIT=$?"
```

### Evidencia de baseline (2026-11-22)

```text
$ pnpm run lint
$ oxlint --deny-warnings
EXIT=0
```

`pnpm run lint 2>&1 | wc -l` → `1` (solo la línea `$ oxlint --deny-warnings`
que imprime pnpm; 0 líneas de diagnóstico). Rama:
`feature/oxlint-zero-warnings`.