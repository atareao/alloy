# Design

## Context

Ver proposal.md para motivación. Estado actual:

- `App.tsx` usa `Layout.Header` con botones `Flex` para navegación (líneas 472-509). En mobile los botones se muestran solo como iconos pero igualmente no caben bien.
- `Layout` principal tiene `border: 'none'` (sin efecto) y `background: var(--ant-color-bg-container)`.
- Dashboard mobile usa `Card bordered` para stacks y `div` sin fondo para filas individuales.
- No hay tests específicos de layout/navegación en el frontend.

## Goals / Non-Goals

**Goals:**
- Layout sin bordes/marcos no deseados en ambos temas
- Sidebar funcional con navegación colapsable
- Dashboard mobile compacto con filas distinguibles

**Non-Goals:**
- No se cambia la lógica de negocio ni el estado de la app
- No se añaden nuevas dependencias externas
- No se modifican tests existentes (solo se añaden nuevos)
- No se cambia el backend

## Decisions

### Decisión 1: Layout.Sider con Menu en vez de botones en Header

**Opción elegida**: `Layout.Sider` de Ant Design con `Menu` y `collapsible`.

**Alternativas consideradas**:
- **Bottom navigation (TabBar)**: Más común en apps mobile nativas, pero Ant Design no tiene un componente nativo para esto y requeriría CSS custom.
- **Drawer con trigger flotante**: Más complejo, menos estándar.
- **Header scrollable (actual)**: No funciona bien, los botones se salen.

**Razón**: `Layout.Sider` es el componente estándar de Ant Design para navegación lateral. Soporta colapso nativo, breakpoints responsive, y drawer en mobile. Es la solución más idiomática con Ant Design.

### Decisión 2: Fondo de filas con `--ant-color-fill-tertiary`

**Opción elegida**: Usar variable CSS de Ant Design `--ant-color-fill-tertiary` para fondo de filas en mobile.

**Alternativas consideradas**:
- `--ant-color-bg-elevated`: Ya se usa en algunos lugares, pero es muy sutil.
- `Card bordered` con padding reducido: Añade bordes que crean el efecto de marco no deseado.

**Razón**: `--ant-color-fill-tertiary` proporciona suficiente contraste con el fondo de página (`--ant-color-bg-container`) sin ser agresivo. Es una variable de tema que se adapta automáticamente a dark/light mode.

### Decisión 3: Sin tests específicos de layout (TDD ligero)

**Opción elegida**: No escribir tests específicos para estos cambios visuales.

**Razón**: Los cambios son puramente visuales (CSS, estructura de componentes). Los tests de componentes con Testing Library no pueden verificar estilos visuales de forma fiable. En su lugar, confiamos en:
- TypeScript (`tsc --noEmit`) para detectar errores de tipos
- Los tests existentes para detectar regresiones funcionales
- Verificación visual manual

## Risks / Trade-offs

- **Sider drawer en mobile**: Ant Design maneja el drawer automáticamente con `breakpoint="md"`, pero puede haber problemas de z-index con otros elementos. → Mitigación: Verificar que el Sider tiene `z-index` adecuado.
- **Regresión en tests existentes**: Los tests de `App.tsx` pueden fallar si la estructura del layout cambia. → Mitigación: Ejecutar `npx vitest run` después de cada cambio.
- **Pérdida de funcionalidad de overflowX**: El header actual tiene `overflowX: 'auto'` que permitía scroll horizontal. Con Sider esto se elimina. → No es un riesgo porque el Sider resuelve el problema de espacio.