# Spec Delta: ui/config-page — ui-pagination-and-vertical-tabs

## ADDED Requirements

### Requirement: Notificaciones y Actualizaciones SHALL usar pestañas verticales

La sección Notificaciones MUST presentar Telegram, Matrix y Valet como pestañas verticales (`Tabs tabPosition="left"`), y la sección Actualizaciones MUST presentar Revisión y Política como pestañas verticales. Los títulos, `aria-label` y placeholders de cada Card MUST permanecer sin cambios; cada Card MUST ser accesible desde su pestaña.

#### Scenario: Pestañas verticales en Notificaciones

- **GIVEN** el usuario en Config → Notificaciones
- **WHEN** se renderiza la sección
- **THEN** las pestañas Telegram, Matrix y Valet aparecen en disposición vertical (a la izquierda)
- **AND** la pestaña activa por defecto es Telegram

#### Scenario: Pestañas verticales en Actualizaciones

- **GIVEN** el usuario en Config → Actualizaciones
- **WHEN** se renderiza la sección
- **THEN** las pestañas Revisión y Política aparecen en disposición vertical (a la izquierda)
- **AND** la pestaña activa por defecto es Revisión

#### Scenario: Cambio entre pestañas internas

- **GIVEN** la sección Notificaciones abierta en Telegram
- **WHEN** el usuario pulsa la pestaña Matrix
- **THEN** se muestra el contenido de MatrixCard
- **AND** TelegramCard deja de estar visible

## MODIFIED Requirements

### Requirement: Comportamiento observable preservado

Salvo la reorganización de Notificaciones y Actualizaciones en pestañas verticales, el resto de la página ConfigPage MUST NOT alterar su comportamiento observable. Las peticiones HTTP, conversiones y guardados MUST usar los mismos endpoints y métodos que antes (`PUT /api/config`, `PUT /api/update-check/config`, `PUT /api/update-policies/default`, `GET /api/update-check/config`, `GET /api/update-check/defaults`, `GET /api/update-policies/default`, `POST /api/notifications/test`, `GET /api/admin/export`, `POST /api/admin/import`), y `check_interval_ms` MUST seguir convirtiéndose de ms a segundos al leer y de segundos a ms al guardar.

#### Scenario: Endpoints y conversiones invariantes

- **GIVEN** el guardado de cada dominio
- **WHEN** se inspecciona la petición HTTP emitida
- **THEN** usa el mismo endpoint y método que antes
- **AND** `check_interval_ms` se convierte igual que antes

#### Scenario: Suite actualizada en verde

- **GIVEN** `frontend/src/components/ConfigPage.test.tsx` actualizado a la nueva estructura de pestañas
- **WHEN** se ejecuta la suite
- **THEN** pasa al 100% en verde
