# Proposal

## Why

`GET /api/config` devuelve los tokens de notificación **en claro**
(`telegram_token`, `matrix_token`, `valet_token`). Cualquier sesión OIDC autenticada puede
leerlos, y la respuesta es fácil de cachear/loguear. Es material secreto que solo debería
usarse en el backend para enviar, no para recargarlo en el navegador.

Es una deuda anotada por el review de `valet-notifier` (el `valet_token`), pero el defecto
es común a los tres tokens; se resuelve de forma consistente para los tres.

## What Changes

- `PublicConfig` deja de exponer los tokens en claro: `telegram_token`, `matrix_token` y
  `valet_token` **se eliminan** del DTO. Se mantienen los indicadores `*_configured`.
- La UI (`ConfigPage`) deja de precargar el token en el campo: el campo empieza **vacío**
  con un `placeholder` que indica que ya hay uno guardado; solo se envía al backend si el
  usuario escribe uno nuevo.
- Semántica de guardado (sin ambigüedad):
  - Campo de token **vacío** + canal activado ⇒ el token **no se envía** ⇒ el backend
    **conserva** el actual.
  - Token escrito ⇒ se envía y se **reemplaza**.
  - Canal **desactivado** ⇒ se envía `""` ⇒ el backend lo **borra** (`None`).
- `UpdateSettingsReq` no cambia (ya distingue "ausente = no tocar" de `""` = borrar).

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities
- `notifications`: la configuración de los canales deja de exponer sus secretos por la API.

## Impact

- `backend/src/models.rs`: quitar `telegram_token`/`matrix_token`/`valet_token` de
  `PublicConfig`.
- `backend/src/config.rs`: `config_handler` deja de rellenar esos campos.
- `frontend/src/types.ts`: quitar los tres campos de `AppConfig`.
- `frontend/src/components/ConfigPage.tsx`: estado de token efímero, placeholders y lógica
  "solo enviar si no está vacío"; `saveTelegram`/`saveMatrix`/`saveValet`.
- `frontend/src/components/ConfigPage.test.tsx`: tests de la nueva semántica.

## Open Questions

- Alternativa rechazada: devolver un token "ofuscado" (`val_••••`). No aporta seguridad (sigue
  siendo reenviable) y complica la comparación. Se devuelve ausente + `*_configured`.
