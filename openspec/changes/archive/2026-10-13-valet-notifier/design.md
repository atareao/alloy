# Design

## Context

Alloy dispara notificaciones desde `notify_all(settings, container, status)`
(`backend/src/notifications.rs`), que hace fan-out en paralelo con `tokio::join!` a
Telegram, Matrix y webhook. Cada canal implementa `notify_*(&Settings, &str, &str) -> Result<(), String>`
y es *best-effort*: un fallo se registra (`ERROR`) y no afecta a los demás.

Los ajustes viven en `Settings` (estructura plana de `Option<String>`/`Option<bool>`,
serializada a una tabla clave-valor en SQLite vía `db::load_settings`/`db::save_settings`)
y se exponen/editan con `GET/PUT /api/config` (`config.rs`).

La API de Valet (ver `/data/rust/valet/docs/api-notifications.md`):

- `POST /api/notifications` — auth `Authorization: Bearer val_…`, `Content-Type: application/json`.
- Cuerpo: `message` (req, ≤4096 B), `origin` (≤64 B, default = nombre de la key),
  `author` (≤128 B, default = nombre de la key), `level` ∈ {`info`,`warning`,`critical`}
  (default `info`), `metadata` (objeto JSON, ≤8192 B).
- Respuestas: `201 {id}`, `400` (validación), `401` (auth), `415` (content-type), `500`.

## Goals / Non-Goals

**Goals**
- Enviar cada cambio de estado de contenedor a Valet con un `level` razonable.
- Configurar URL + token desde la UI como el resto de canales.
- No romper ni bloquear Telegram/Matrix/webhook (best-effort, timeout acotado).
- Fallar de forma visible en logs cuando Valet está configurado pero rechaza la petición.

**Non-Goals**
- Gestionar ingest keys, leer la bandeja ni emitir WebPush (eso es de Valet).
- Mapeo de niveles configurable (se propone fijo).
- Reintentos/cola persistente: si Valet está caído, se pierde esa notificación (igual que
  el resto de canales).

## Decisions

### 1. Dos ajustes: `valet_url` + `valet_token`
`valet_url` es la base (sin `/api/notifications`); se normaliza quitando `/` finales. La
petición va a `{base}/api/notifications`. `valet_configured` es `true` solo si **ambos**
están presentes (a diferencia de Telegram/Matrix, que solo miran un campo).

*Alternativa descartada*: reutilizar el webhook genérico apuntándolo a Valet — el webhook
envía `{event, container, status, timestamp, source}` y Valet espera
`{message, origin, author, level, metadata}`; reutilizarlo obligaría a un transformador o a
un formato dual. Un canal dedicado es más claro y testeable.

### 2. Cuerpo enviado
```json
{
  "message": "🪐 Alloy: {container} → {status}",
  "origin": "alloy",
  "author": "alloy",
  "level": "info|warning|critical",
  "metadata": { "container": "{container}", "status": "{status}", "timestamp": "<rfc3339>" }
}
```
`origin`/`author` fijos a `"alloy"` (no dependemos del nombre de la key). `metadata` con el
detalle estructurado (dentro del límite de 8192 B; el timestamp va corto).

### 3. Mapeo de nivel (determinista, minúsculas) — sobre el estado CRUDO
Función pura `fn valet_level(state: &str) -> &'static str`:

- `critical` si el estado contiene `died`, `dead` o `error`.
- `warning` si contiene `exit` (exited) o `stop` (stopped).
- `info` en cualquier otro caso (`running`, `start`, `pause`, `unpause`, `restart`, `create`…).

El matching es por subcadena en minúsculas sobre el **estado crudo de Docker**
(`c.state`: `running`, `exited`, `dead`, `paused`, `restarting`, `created`…) y NO sobre el
mensaje visible. Pura → testeable sin red.

> [!important] Por qué sobre el estado crudo
> El `status` que hoy viaja a `notify_all` desde el state worker es
> `status_message(&c.state)`, es decir la **etiqueta ya localizada** al español
> (`"▶️ en ejecución"`, `"⏹️ detenido"`, `"💀 finalizado"`). Si `valet_level` mirase ese
> mensaje, **todos** los eventos reales saldrían como `info`, incluido un contenedor
> `dead`. Para que el nivel tenga sentido, la capa que sí conoce el estado crudo (el state
> worker) debe propagarlo explícitamente.

### 3b. Propagación del estado crudo al fan-out
Se añade `notify_all_state(settings, container, status_msg, raw_state: Option<&str>)`:
calcula `level = raw_state.map(valet_level).unwrap_or("info")` y se lo pasa a
`notify_valet`. `notify_all` (firma actual) delega en `notify_all_state` con
`raw_state = None`, de modo que los avisos basados en **mensaje** (actualizaciones, stacks,
scheduler) usan `info` — que es su semántica correcta — y **no hay que tocar sus 6 call
sites**.

`detect_notifications` pasa a devolver `(name, status_message, raw_state)` para que el
state worker pueda invocar `notify_all_state` con el estado crudo. `notify_valet` recibe el
`level` ya calculado (sigue siendo `valet_level` la función pura testeable).

### 4. Integración en `notify_all`
```rust
tokio::join!(
    notify_telegram(...),
    notify_matrix(...),
    notify_webhook(...),
    notify_valet(...)
)
```
Igual que el resto: no propaga errores, no aborta a los demás.

### 5. Endpoint de test
`POST /api/notifications/test` con `{"channel":"valet"}` reutiliza `notify_valet(&s,
"Alloy Test", "🧪 …")` y devuelve `{status:"ok", channel:"valet", message:"enviado"}` o
`AppError::Internal`. El mensaje de canal desconocido pasa a mencionar `valet`.

### 6. Persistencia y API de config
- `db.rs`: añadir `valet_url`/`valet_token` al mapa de `load_settings`/`save_settings`
  (mismo patrón que `webhook_url`, filtrando cadenas vacías).
- `models.rs`: campos en `Settings` (`#[serde(default)] Option<String>`) y
  `UpdateSettingsReq`; campos en `PublicConfig`.
- `config.rs`: `valet_configured = url.is_some() && token.is_some()`; en `update_config_h`,
  cadena vacía ⇒ `None` (desactivar).

### 7. Frontend
Tarjeta "🪐 Valet" en `ConfigPage` (pestaña Notificaciones), calcada de la de Matrix:
switch + `Input` para URL + `Input.Password` para token + *Guardar* + *Test*. `saveValet`
hace `PUT /api/config` con `{valet_url, valet_token}` (o cadenas vacías si el switch está
apagado). `types.ts` añade los tres campos.

## Risks / Trade-offs

- **Token en `PublicConfig`**: consistente con el resto pero expone el secreto a la UI.
  Mitigación futura: redactar tokens en `GET /api/config`. Fuera de alcance aquí.
- **Pérdida de notificaciones si Valet cae**: aceptado (best-effort, igual que los demás).
- **`level` fijo**: si en el futuro se quiere distinguir por política de contenedor, hay
  que añadir ajuste. Se documenta como extensión.
- **Doble API `notify_all` / `notify_all_state`**: mínima superficie nueva (una función) a
  cambio de no tocar los 6 call sites existentes. Descartado cambiar la firma de
  `notify_all` (más invasivo sin ganancia).
- **URL mal formada**: se normaliza `/` final; una URL no válida producirá `Err` de conexión
  registrado, sin panic.

## Migration Plan

- Sin migración de datos: los ajustes van a la tabla clave-valor existente; los campos
  nuevos son opcionales y ausentes ⇒ desactivado por defecto.
- Despliegue: reconstruir backend+frontend y recrear el contenedor (compose pull + up -d).
- Rollback: revertir la imagen; los campos `valet_*` en la tabla de ajustes son inertes.

## Open Questions

- ¿Mapeo de niveles configurable? (propuesta: no, fijo).
- ¿Redactar `valet_token` en `GET /api/config`? (propuesta: no, consistencia con el resto).
