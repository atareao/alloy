# Tasks

## 1. RED — Test que falla

- [x] 1.1 `ConfigPage.test.tsx`: test que obtiene por rol/nombre accesible el switch
      ("Activar Valet") y los campos "URL de Valet" e "Ingest key de Valet".
- [x] 1.2 `npx vitest run` en rojo (los demás verdes).

## 2. GREEN — Implementación mínima

- [x] 2.1 `ConfigPage.tsx`: `aria-label="Activar Valet"` en el `<Switch>` de Valet.
- [x] 2.2 `ConfigPage.tsx`: `aria-label="URL de Valet"` y `aria-label="Ingest key de Valet"`
      en los dos `<Input>`.
- [x] 2.3 `npx vitest run` verde + `npx tsc -b`.

## 3. REFACTOR — Verificación

- [x] 3.1 `npm run lint` sin errores.
- [x] 3.2 `npx vitest run` sin regresiones.
- [x] 3.3 Revisión con `react-reviewer`.

## 4. Archive

- [x] 4.1 `openspec validate valet-a11y --strict`
- [x] 4.2 `openspec archive valet-a11y --yes`
- [x] 4.3 Verificar/eliminar `openspec/changes/valet-a11y/`
