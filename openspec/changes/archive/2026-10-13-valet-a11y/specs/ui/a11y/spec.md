# Spec Delta

## Purpose

Garantizar que la tarjeta de configuración de **Valet** en `ConfigPage` es operable con
lectores de pantalla: sus controles tienen nombre accesible.

## ADDED Requirements

### Requirement: Controles accesibles de la tarjeta Valet

El `<Switch>` de activación y los campos de URL e ingest key de la tarjeta Valet MUST exponer
un nombre accesible (`aria-label`), de forma que un lector de pantalla anuncie su propósito
y no solo el estado o el `placeholder`.

#### Scenario: El switch tiene nombre accesible

- **Given** la tarjeta Valet renderizada
- **When** un lector de pantalla enfoca el switch
- **Then** se anuncia con el nombre "Activar Valet"

#### Scenario: Los campos tienen nombre accesible

- **Given** la tarjeta Valet activada
- **When** un lector de pantalla enfoca el campo de URL o el de la ingest key
- **Then** se anuncia respectivamente "URL de Valet" e "Ingest key de Valet"
