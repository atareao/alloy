# Change Proposal: ui-fixes-v4

## Intent
Hacer que las Cards de stacks en modo mobile se vean como las cards de stats (Total, Running, Stopped) — con fondo contrastado (`bg-container`), bordes visibles, esquinas redondeadas y separación entre cards.

## Why
En `ui-fixes-v3` se restauró el borde por defecto de Ant Design (`bordered={true}`). Pero las cards siguen sin parecer cards porque:
1. El inner div tiene `background: var(--ant-color-bg-layout)`, que es el **mismo color que el fondo de la página** → la Card se funde
2. El grid tiene `gap: 1` → las cards están pegadas, sin separación visual
3. Las esquinas redondeadas no se aprecian porque la Card se confunde con el fondo

## What Changes
1. **DashboardPage.tsx** — `renderGroup` mobile (línea 334): cambiar `background: "var(--ant-color-bg-layout)"` a `background: "transparent"` para que el fondo nativo de la Card (`bg-container`) se vea
2. **ContainerTable.tsx** — grid mobile (línea 140): cambiar `gap: 1` a `gap: 8` para que haya separación visible entre cards

## Impact
- Solo visual, mobile exclusivamente
- La Card usará su fondo por defecto (`var(--ant-color-bg-container)`), distinto del fondo de página (`var(--ant-color-bg-layout)`)
- Las cards tendrán 8px de separación entre sí
- Las esquinas redondeadas del Card serán visibles
- Sin cambios en tests ni lógica