# ui/config-page Specification

## ADDED Requirements

### Requirement: ConfigPage como orquestador delgado

`ConfigPage` MUST limitarse a orquestar la página: renderizar las pestañas (`Tabs`), las alertas de error/éxito de nivel superior y componer las Cards. MUST NOT contener el estado de formulario por dominio ni los handlers de red por dominio.

#### Scenario: Composición de la página

- **Given** la página de Configuración renderizada
- **When** se inspecciona su árbol de componentes
- **Then** muestra 3 pestañas: `Notificaciones`, `Actualizaciones` e `Información`
- **And** cada card (Telegram, Matrix, Valet, Revisión de actualizaciones, Política por defecto, Información de la aplicación, Tema, Exportar/Importar) es un componente en `src/components/`

#### Scenario: Tamaño del orquestador

- **Given** el fichero `src/components/ConfigPage.tsx`
- **When** se mide su tamaño
- **Then** es sustancialmente menor que el original (1698 líneas) y no contiene ningún handler `save*`

### Requirement: Validación de cron en un módulo puro

La validación y normalización de expresiones cron (`normalizeCron`, `isValidCronClient`, `cronErrorFor`, `CRON_FIELDS`, etc.) MUST vivir en un módulo puro sin estado ni dependencias de React, testeable de forma aislada.

#### Scenario: Validación aislada

- **Given** una expresión cron de entrada
- **When** se valida con el módulo puro
- **Then** devuelve el mismo resultado (válida/inválida y mensaje de error) que el comportamiento previo
- **And** el módulo no importa React

### Requirement: Estado compartido de la página en un hook

El estado compartido de la página (`saving`, `testing`, `error`, `success`, `activeTab`) y el auto-ocultado del mensaje de éxito MUST residir en un hook dedicado reutilizado por `ConfigPage` y sus hijos.

#### Scenario: Auto-ocultado del mensaje de éxito

- **Given** un guardado completado con éxito
- **When** transcurren 3 segundos
- **Then** el mensaje de éxito deja de mostrarse
- **And** no se produce `setState` tras desmontar el componente (el timer se limpia)

#### Scenario: Estado de guardado por clave

- **Given** un guardado en curso de un canal concreto
- **When** se consulta el estado de guardado
- **Then** se identifica por la misma clave previa (`"telegram"`, `"matrix"`, `"valet"`, `"update-check"`, `"default-policy"`)

### Requirement: Comportamiento observable preservado

El refactor MUST NOT alterar ningún comportamiento observable de la página.

#### Scenario: Suite existente intacta

- **Given** `frontend/src/components/ConfigPage.test.tsx` **sin modificar**
- **When** se ejecuta la suite tras el refactor
- **Then** pasa al 100% en verde

#### Scenario: Endpoints y conversiones invariantes

- **Given** el guardado de cada dominio
- **When** se inspecciona la petición HTTP emitida
- **Then** usa el mismo endpoint y método que antes (`PUT /api/config`, `PUT /api/update-check/config`, `PUT /api/update-policies/default`, `GET /api/update-check/config`, `GET /api/update-check/defaults`, `GET /api/update-policies/default`, `POST /api/notifications/test`, `GET /api/admin/export`, `POST /api/admin/import`)
- **And** `check_interval_ms` se convierte de ms a segundos al leer y de segundos a ms al guardar, igual que antes

#### Scenario: Contratos accesibles preservados

- **Given** cada Card renderizada
- **When** se localiza por su título visible (helper `getCard` de los tests)
- **Then** el título, los `aria-label` y los placeholders no han cambiado

#### Scenario: Typecheck y lint limpios

- **Given** el refactor aplicado
- **When** se ejecutan `tsc -b`, `lint` y `build` en `frontend/`
- **Then** terminan sin errores ni warnings nuevos
