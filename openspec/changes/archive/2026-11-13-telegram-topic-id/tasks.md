# Tasks

## 1. Red — tests que deben fallar primero

- [x] 1.1 Backend: en `notifications.rs`, test que construya el payload y exija
      `message_thread_id: 42` con topic `"42"`, y ausencia de la clave sin topic.
- [x] 1.2 Backend: test de topic inválido (`"abc"`) → `notify_telegram` devuelve `Err`.
- [x] 1.3 Backend: en `db.rs`, test round-trip `telegram_topic_id` (guardar/cargar, incluido `None`).
- [x] 1.4 Backend: en `config.rs`, tests de `PUT` (guarda, borra con `""`, conserva si ausente)
      y `GET` devuelve `telegram_topic_id`.
- [x] 1.5 Frontend: en `ConfigPage.test.tsx`, tests de precarga del campo, envío en `PUT` y
      limpieza (`""`) al apagar el switch.
- [x] 1.6 Ejecutar y confirmar RED: `cd backend && cargo test` y `cd frontend && pnpm run test`
      fallan por los nuevos casos.

## 2. Green — modelo y persistencia

- [x] 2.1 `models.rs`: `Settings.telegram_topic_id: Option<String>` (`#[serde(default)]`).
- [x] 2.2 `models.rs`: `PublicConfig.telegram_topic_id: Option<String>`.
- [x] 2.3 `models.rs`: `UpdateSettingsReq.telegram_topic_id: Option<String>`.
- [x] 2.4 `db.rs`: `load_settings` lee `telegram_topic_id` (vacío ⇒ `None`).
- [x] 2.5 `db.rs`: `save_settings` guarda/borra `telegram_topic_id` como el resto.

## 3. Green — API y envío

- [x] 3.1 `config.rs`: `config_handler` expone `telegram_topic_id`.
- [x] 3.2 `config.rs`: `update_config_h` aplica semántica ausente/vacío/valor con `trim`.
- [x] 3.3 `notifications.rs`: helper `tg_topic_id` y refactor del cuerpo a una función
      testeable `telegram_payload(chat_id, text, topic)`; incluye `message_thread_id` (número)
      solo si `Some`.
- [x] 3.4 `notifications.rs`: topic presente no numérico ⇒ `Err` sin HTTP + `ERROR`.

## 4. Green — frontend

- [x] 4.1 `types.ts`: `telegram_topic_id: string | null` en `AppConfig`.
- [x] 4.2 `ConfigPage.tsx`: estado `tgTopicId`, precarga desde props, input "Topic ID (opcional)".
- [x] 4.3 `ConfigPage.tsx`: incluir `telegram_topic_id` en el `PUT` con `trim`, enviando `""`
      (no `null`) cuando el campo queda vacío; `""` también al apagar el switch.
- [x] 4.4 `ConfigPage.tsx`: input numérico (`inputMode="numeric"`, solo dígitos) para no guardar
      valores que romperían el envío.

## 5. Verificación

- [x] 5.1 `cd backend && cargo test` verde.
- [x] 5.2 `cd backend && just check` verde (fmt + clippy `-D warnings`).
- [x] 5.3 `cd frontend && pnpm exec tsc -b && pnpm run test && pnpm run build && pnpm run lint` verdes.
- [x] 5.4 Test manual: `POST /api/notifications/test` con topic configurado y comprobar en
      Telegram que cae en el tema correcto (o inspección del payload en logs/test).

## 6. Docs y cierre

- [x] 6.1 `docs/telegram-topic-id.md` (qué es, ficheros, cómo funciona, configuración, follow-ups).
- [x] 6.2 Actualizar tabla de variables/README y `alloy.container.example` si procede.
- [x] 6.3 Archivar: `openspec archive telegram-topic-id --yes`; verificar limpieza del change dir.
