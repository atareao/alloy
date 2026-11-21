# apply-default-guard-overrides

Issue: #155.

## Qué es

Hace efectivos los **overrides de la guarda** de la **política por defecto**
(`max_start_attempts`, `health_window_secs`). Antes se enviaban desde la UI pero el backend
los **descartaba en silencio** (no-op).

## Problema (antes)

- La sección «🛡️ Overrides de la guarda» de la política por defecto (`ConfigPage.tsx`) enviaba
  ambos campos en `PUT /api/update-policies/default`, pero `DefaultUpdatePolicy` (`admin.rs`)
  solo tenía 3 campos → **serde los ignoraba sin error**.
- Los contenedores **sin** política se resolvían con una `UpdatePolicy` sintética con esos campos
  a `None` (`scheduler.rs`, `updates/handlers.rs`), así que caían siempre al global.

## Cómo funciona (después)

- **`Settings`**: nuevos `default_max_start_attempts: Option<u32>` y
  `default_health_window_secs: Option<u64>` (`#[serde(default)]`; los settings previos cargan
  con `None`, sin migración). Helper `Settings::default_guard_overrides()`.
- **`GET/PUT /api/update-policies/default`**: devuelve y persiste ambos overrides
  (`null`/ausente = sin override).
- **Aplicación**: los tres sitios que construyen la `UpdatePolicy` de *fallback* para un
  contenedor **sin** política siembran sus overrides desde `settings.default_guard_overrides()`.

### Precedencia

```
override de política de contenedor  >  override de política por defecto  >  global
```

Los overrides de la política por defecto aplican **solo a contenedores sin política específica**.
Un contenedor **con** política que no fije su override propio sigue cayendo al **global** (como
declara su UI: «Vacío = usar el valor global»).

## Ficheros implicados

- `backend/src/models.rs`: campos en `Settings` + helper.
- `backend/src/db.rs`: persistencia/lectura de los dos campos en `load_settings`/`save_settings`.
- `backend/src/admin.rs`: `DefaultUpdatePolicy` (GET/PUT).
- `backend/src/updates/handlers.rs`: seed del fallback (×2).
- `backend/src/workers/scheduler.rs`: seed del fallback.

## Fuera de alcance

- Flujo manual monovolumen `update_container_h`: usa los globales por diseño documentado (no
  resuelve política por contenedor).
- Contrato `UpdateCheckConfig` (`/api/update-check/config`): sin cambios.
- Frontend: sin cambios (ya enviaba y cargaba estos campos).

## Verificación

- Tests: round-trip `PUT`/`GET`; precedencia (contenedor > default > global); compat de settings
  sin los campos.
- `cd backend && just check` limpio y `cargo test` en verde.

## Caveats / follow-ups

- Si en el futuro se quiere que el override por defecto cubra también a contenedores **con**
  política que no fijen su propio override (cascada), habría que tocar `guard_params` y el texto
  de la UI de política por contenedor.
