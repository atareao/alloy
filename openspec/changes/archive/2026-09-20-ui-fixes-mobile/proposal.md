# Proposal

## Why

The Alloy dashboard has three visual issues that degrade the user experience: (1) a visible frame/border around the entire UI in both dark and light themes, (2) the navigation tabs (Dashboard, Historial, Config, Salir) overflow outside the viewport on mobile devices, and (3) the mobile Dashboard view has excessive gaps between container rows with no visual distinction between cells and background.

## What Changes

- Remove redundant borders and frames from the root Layout and nested Card components
- Fix header overflow on mobile by reducing spacing and allowing wrapping
- Compact mobile Dashboard rows by removing Card wrappers, reducing gaps, and adding subtle row separators
- Make container cells visually distinct from the background on mobile

## Capabilities

### New Capabilities
*(none — this is a pure UI/CSS refactor with no behavior changes)*

### Modified Capabilities
*(none — no spec-level behavior changes)*

## Impact

- **Files modified:**
  - `frontend/src/App.tsx` — Header overflow fix, Layout border removal
  - `frontend/src/components/ContainerRow.tsx` — Remove Card border on mobile, compact padding
  - `frontend/src/components/ContainerTable.tsx` — Compact mobile grid, remove Card wrapper for "Sin stack" section
  - `frontend/src/components/DashboardPage.tsx` — Stats cards border cleanup, stack group padding
- **No API changes, no backend changes, no dependency changes**
- **No behavior changes** — purely visual/styling fixes