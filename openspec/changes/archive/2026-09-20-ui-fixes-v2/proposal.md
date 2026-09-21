# Change Proposal: ui-fixes-v2

## Intent
Corregir dos problemas en las celdas del grid mobile de stacks en el dashboard:

1. **Altura de celda**: El grid cell tiene `aspectRatio: "1"` (cuadrado forzado), pero la `Card` interna no tiene `height: 100%`, por lo que la Card no llena el cuadrado y queda espacio vacío sobrante.
2. **Rejilla invisible**: El grid usa `background: var(--ant-color-bg-container)` con `gap: 1px`, pero como las celdas también usan fondos similares (`bg-layout`), la separación de 1px es invisible y las celdas se funden con el fondo.

## Scope
- `frontend/src/components/ContainerTable.tsx` — cambiar `background` del grid y añadir `height: 100%` a la Card
- `frontend/src/components/DashboardPage.tsx` — añadir `height: 100%` a la Card en `renderGroup` mobile
- No se tocan otros archivos ni componentes

## Impact
- **Visual**: Las celdas del grid mobile llenan todo el cuadrado sin huecos, y se distinguen entre sí gracias a líneas de rejilla visibles
- **Temas**: Funciona tanto en tema claro como oscuro (usa variables CSS temáticas)
- **Tests**: Sin cambios en tests existentes (solo cambios de estilo/variables CSS)
- **Sin regresiones**: No afecta al layout desktop ni a otras vistas

## Contracts

### ContainerTable.tsx (línea 141)
```diff
- background: 'var(--ant-color-bg-container)',
+ background: 'var(--ant-color-border-secondary)',
```

### DashboardPage.tsx — renderGroup mobile (Card)
Añadir `style={{ height: '100%' }}` a la Card en el bloque `if (isMobile)` de `renderGroup`.