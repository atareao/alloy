# notifications Specification

## Purpose
Definir los canales de notificación salientes de Alloy y, en concreto, el nuevo canal
**Valet**: cómo se configura, cómo se mapea el nivel y que el envío es *best-effort* y no
bloquea a los demás canales.

## Requirements

### Requirement: Notificador Valet

Alloy MUST enviar los cambios de estado de contenedor a la API de ingesta de Valet mediante
`POST {valet_url}/api/notifications`, autenticado con `Authorization: Bearer {valet_token}`
y `Content-Type: application/json`. `valet_url` MUST normalizarse eliminando las barras
finales (y los espacios de los extremos). El canal MUST considerarse configurado solo cuando
`valet_url` **y** `valet_token` estén presentes; si falta cualquiera de los dos,
`notify_valet` MUST devolver `Err` sin emitir petición HTTP.

#### Scenario: Envío correcto a Valet

- **Given** `valet_url` y `valet_token` configurados y un Valet que responde `201`
- **When** se produce un cambio de estado de contenedor y se ejecuta `notify_valet`
- **Then** se hace `POST` a `{valet_url}/api/notifications` con cabecera
  `Authorization: Bearer {valet_token}`
- **And** el cuerpo contiene `message`, `origin:"alloy"`, `author:"alloy"`, `level` y
  `metadata` con `container`, `status` y `timestamp`
- **And** `notify_valet` devuelve `Ok(())` y registra un `INFO`

#### Scenario: Valet no configurado

- **Given** que falta `valet_url` o `valet_token`
- **When** se ejecuta `notify_valet`
- **Then** devuelve `Err` con un mensaje "Valet no configurado"
- **And** NO se emite ninguna petición HTTP

#### Scenario: Valet rechaza la notificación

- **Given** `valet_url` y `valet_token` configurados y un Valet que responde `401` o `400`
- **When** se ejecuta `notify_valet`
- **Then** devuelve `Err` con el código HTTP y el cuerpo de la respuesta
- **And** se registra un `ERROR`

#### Scenario: Error de conexión con Valet

- **Given** que `valet_url` no es alcanzable
- **When** se ejecuta `notify_valet`
- **Then** devuelve `Err` con la causa de conexión
- **And** se registra un `ERROR`

### Requirement: Mapeo de nivel de Valet

El campo `level` del cuerpo MUST derivarse del **estado crudo de Docker** del contenedor
(`running`, `exited`, `dead`, `paused`, `restarting`, `created`…), de forma determinista y
sin depender de la red. MUST NOT derivarse del mensaje visible localizado. El valor MUST ser
uno de `info`, `warning` o `critical` (los aceptados por Valet). Los avisos que no
corresponden a un estado de contenedor (actualizaciones, stacks, scheduler) MUST usar
`info`.

#### Scenario: Estado crítico

- **Given** un `status` que contiene `died`, `dead` o `error`
- **When** se calcula el nivel
- **Then** el nivel es `critical`

#### Scenario: Estado de parada

- **Given** un `status` que contiene `exit` o `stop` (y no coincide con crítico)
- **When** se calcula el nivel
- **Then** el nivel es `warning`

#### Scenario: Estado normal

- **Given** un `status` normal (`running`, `start`, `pause`, `unpause`, `restart`, `create`…)
- **When** se calcula el nivel
- **Then** el nivel es `info`

#### Scenario: Nivel calculado desde el estado crudo, no desde el mensaje localizado

- **Given** un contenedor que transiciona a `exited`
- **When** el state worker emite la notificación (mensaje visible `"⏹️ detenido"`)
- **Then** el `level` enviado a Valet es `warning` (calculado sobre `"exited"`, no sobre el
  mensaje localizado)
- **And** un contenedor que transiciona a `dead` produce `level` `critical`
- **And** un contenedor que transiciona a `running` produce `level` `info`

#### Scenario: Aviso basado en mensaje usa info

- **Given** una notificación de actualización con mensaje `"✅ actualizado y reiniciado"`
- **When** se envía a Valet
- **Then** el `level` es `info`

### Requirement: Valet en el fan-out best-effort

`notify_all` MUST incluir `notify_valet` en paralelo con Telegram, Matrix y webhook. Un
fallo de Valet MUST NOT impedir ni alterar el envío por los demás canales.

#### Scenario: Fallo de Valet no afecta a los demás canales

- **Given** Telegram/Matrix configurados y Valet caído
- **When** se ejecuta `notify_all`
- **Then** Telegram y Matrix reciben su notificación con normalidad
- **And** el fallo de Valet solo se registra en logs

### Requirement: Configuración de Valet en la API y la UI

Los ajustes `valet_url` y `valet_token` MUST persistirse en SQLite y exponerse y editarse a
través de `GET/PUT /api/config`. `PublicConfig` MUST incluir `valet_configured`,
`valet_url` y `valet_token`. En `PUT /api/config`, una cadena vacía MUST desactivar el
ajuste (`None`). La UI MUST ofrecer una tarjeta Valet con switch, URL, token, guardar y
test.

#### Scenario: Guardar ajustes de Valet

- **Given** un usuario con sesión que envía `{"valet_url":"https://v.test","valet_token":"val_x"}`
- **When** se llama a `PUT /api/config`
- **Then** los ajustes se guardan en SQLite
- **And** la respuesta incluye `valet_configured:true`, `valet_url` y `valet_token`

#### Scenario: Desactivar Valet

- **Given** un usuario que envía `{"valet_url":"","valet_token":""}`
- **When** se llama a `PUT /api/config`
- **Then** ambos ajustes pasan a `None` y `valet_configured` es `false`

#### Scenario: Test de Valet desde la UI

- **Given** Valet configurado
- **When** se llama a `POST /api/notifications/test` con `{"channel":"valet"}`
- **Then** se envía un mensaje de prueba vía `notify_valet` y se responde
  `{status:"ok", channel:"valet", message:"enviado"}`

#### Scenario: Canal desconocido incluye Valet

- **Given** un `channel` no soportado
- **When** se llama al endpoint de test
- **Then** se responde `400` con un mensaje que enumera `telegram`, `matrix` y `valet`

### Requirement: El fan-out no retiene el lock de ajustes

`notify_all_state` MUST leer los ajustes una sola vez y MUST liberar el `Mutex<Settings>`
antes de iniciar las llamadas de red de los canales. Mientras un fan-out está en curso,
`GET/PUT /api/config` y el endpoint de test MUST poder adquirir el lock sin esperar a que
termine el envío.

#### Scenario: La config no se bloquea durante el fan-out

- **Given** un fan-out de notificación en curso (una llamada de canal en vuelo)
- **When** se intenta adquirir el lock de `Settings`
- **Then** la adquisición tiene éxito sin esperar a que termine el canal

### Requirement: Los secretos de notificación no se exponen por la API

`GET /api/config` y la respuesta de `PUT /api/config` MUST NOT incluir los secretos de los
canales de notificación en claro: `telegram_token`, `matrix_token` y `valet_token` MUST estar
ausentes del payload. La API MUST exponer únicamente indicadores booleanos `*_configured`.
`PUT /api/config` MUST distinguir "campo de token ausente" (conservar el actual) de cadena
vacía (borrarlo), y la UI MUST permitir guardar sin reenviar el token cuando el campo se deja
vacío.

#### Scenario: Lectura de la config sin secretos

- **Given** canales configurados con sus tokens
- **When** se llama a `GET /api/config`
- **Then** la respuesta NO contiene `telegram_token`, `matrix_token` ni `valet_token`
- **And** sí contiene `telegram_configured`, `matrix_configured` y `valet_configured` en `true`

#### Scenario: Guardar sin reenviar el token lo conserva

- **Given** un canal activado con token guardado y el campo de token vacío en la UI
- **When** se hace `PUT /api/config` sin el campo de token
- **Then** el token existente se conserva y el canal sigue configurado

#### Scenario: Escribir un token lo reemplaza

- **Given** un canal con token guardado
- **When** se hace `PUT /api/config` con un token nuevo no vacío
- **Then** el token se reemplaza por el nuevo

#### Scenario: Desactivar borra el token

- **Given** un canal activado
- **When** se hace `PUT /api/config` con el token a `""`
- **Then** el token pasa a `None` y `*_configured` pasa a `false`
