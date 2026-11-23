# Spec Delta: frontend/update-history — ui-pagination-and-vertical-tabs

## ADDED Requirements

### Requirement: El tamaño de página por defecto SHALL ser 10

`HistoryPage` MUST inicializarse con un tamaño de página de 10 (constante `DEFAULT_HISTORY_PAGE_SIZE`), manteniendo el selector con las opciones 10, 20 y 50.

#### Scenario: Carga inicial con 10

- **GIVEN** un histórico con 25 entradas
- **WHEN** se abre el histórico
- **THEN** la petición inicial usa `/api/history?page=1&page_size=10`
- **AND** se muestran 10 filas en la página 1
