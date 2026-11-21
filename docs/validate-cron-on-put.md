# validate-cron-on-put

Pendiente derivado de #125 (`update-rollback-guard`) / #135. Issue: #137.

## Qué es

Evita que se persista una configuración de updates con una **expresión cron inválida**. Hasta ahora
`PUT /api/update-check/config` (y `POST /api/admin/import`) guardaban el cron sin validarlo; un cron
inválido quedaba almacenado y el scheduler lo ignoraba en silencio (`warn!`), dejando config rota sin
feedback.

## Decisiones (operador)

- Cron **inválido** → **400** `{"error": msg}`; no se persiste ni se muta el estado.
- Formato canónico: **solo 5 campos** (`0 6 * * *`). Un input de 6 campos → 400.
- Cron **vacío** → 400 en el PUT; en el **import**, `None`/`""` significan "sin override" y se aceptan.

## Ficheros implicados

- `backend/src/timezone.rs` — nuevo `pub fn is_valid_cron(expr: &str) -> bool`: rechaza vacío/whitespace,
  exige `split_whitespace().count() == 5` y valida `format!("0 {expr}").parse::<cron::Schedule>()`
  (la crate `cron` 0.15 espera 6 campos; la app usa 5 y prefija `"0 "`). **Punto único** de validación.
- `backend/src/admin.rs`:
  - `put_update_check_config_h` → `Result<Json<UpdateCheckConfig>, AppError>`; valida `body.cron` **antes**
    de tomar el lock / mutar / persistir; `AppError::BadRequest` en inválido/vacío/6-campos.
  - `import_config_h` → `Result<Json<Value>, AppError>`; valida `update_check_cron` **antes** de aplicar el
    `Settings` (solo si `Some` no vacío); `None`/`""` se aceptan.
- `frontend/src/components/ConfigPage.tsx` — `saveUpdateCheck`: en `!res.ok` lee el body del error y
  muestra `data.error` en el `Alert` (antes, mensaje genérico sin leer la respuesta).

## Cómo funciona

El PUT/import devuelven `AppError::BadRequest` (400 con `{"error": msg}`) cuando el cron no pasa
`is_valid_cron`. El frontend propaga ese mensaje al usuario. El camino válido no cambia (200 + cálculo de
`next_run_at`). Al rechazar, el estado en memoria y el persistido quedan intactos.

## Variables / config

- `DEFAULT_UPDATE_CHECK_CRON = "0 6 * * *"` (`models.rs`): default cuando no hay cron.

## Cómo habilitar / desplegar

Sin acción: la validación es automática en los endpoints de escritura.

## Verificación

- Backend: `cargo test` → **430 OK**; `just check` limpio.
- Frontend: `npx vitest run` → **103 OK**; `pnpm exec tsc -b` y `pnpm run lint` limpios.
- Auditorías: `@rust-reviewer` (helper + PUT + import) y `@react-reviewer` → **APROBADO**.

## Caveats / follow-ups

- **Validación cliente** (regex/previa) del cron en el formulario: no incluida.
- **Normalizar inputs de 6 campos** (aceptarlos y evitar el doble prefijo): no incluido.
- `compute_next_run`/`next_cron_time` duplican el prefijado `"0 "` (cálculo, no validación): refactor opcional.
