# Change Proposal: ui-fixes-v3

## Intent
Hacer que las Cards de stacks en modo mobile se vean como Cards de verdad — con borde visible, fondo contrastado y aspecto de tarjeta independiente.

## Why
En la iteración anterior (`ui-fixes-v2`) se corrigió el hueco sobrante (`height: 100%`) y la rejilla invisible (`border-secondary`). Pero las Cards siguen con `bordered={false}`, lo que las hace ver como manchas planas sin bordes. El usuario quiere que parezcan cards.

## What Changes
1. **DashboardPage.tsx** — `renderGroup` mobile: eliminar `bordered={false}` del Card (por defecto Ant Design aplica borde y estilo de card)
2. **ContainerTable.tsx** — grid mobile: cambiar `background` a `transparent` para que no compita con los bordes de las cards

## Impact
- Solo visual, mobile exclusivamente
- Usa el estilo por defecto de Ant Design Card (borde 1px, border-radius, bg-container)
- Sin cambios en tests ni lógica