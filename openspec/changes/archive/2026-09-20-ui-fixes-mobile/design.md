# Design

## Context

The Alloy frontend uses Ant Design v6 components with inline styles (no CSS files). The three visual issues stem from:

1. **Frame/border**: Ant Design `Card` components default to `bordered=true`, creating visible borders around every section. The root `Layout` also has default styling that can produce a visible outline.
2. **Header overflow**: The header uses `Flex wrap="nowrap"` with 4 buttons + logo + username text. On mobile (<768px) this doesn't fit.
3. **Mobile gaps**: Each `ContainerRow` is wrapped in a `Card bordered size="small"`, and the "Sin stack" section wraps them in another `Card`. This creates nested borders and excessive spacing.

## Goals / Non-Goals

**Goals:**
- Remove visible frame/border around the entire app in both themes
- Make navigation tabs fit within the viewport on mobile
- Compact mobile Dashboard rows — flush against each other with subtle visual separation
- Make container cells visually distinct from the background

**Non-Goals:**
- No behavior changes to container management logic
- No changes to desktop layout (only mobile-specific adjustments)
- No CSS files or external stylesheets — all changes via inline styles
- No changes to Ant Design theme configuration

## Decisions

### Decision 1: Remove Card borders from ContainerRow on mobile
- **Approach**: Conditionally render `Card` vs plain `div` based on `isMobile`. On mobile, use a `div` with `borderBottom` instead of a full Card.
- **Rationale**: Cards have padding, margin, and border that create visual gaps. A simple `div` with bottom border is visually lighter and rows stack flush.
- **Alternative considered**: Using `Card bordered={false}` — still has Card padding and background, doesn't solve the gap issue.

### Decision 2: Header overflow — reduce spacing and allow wrapping
- **Approach**: On mobile, reduce `marginLeft` on username, reduce button gaps, and set `overflowX: 'auto'` on the button container.
- **Rationale**: Minimal changes, preserves all buttons visible via scroll if needed.
- **Alternative considered**: Hiding username on mobile — loses useful info. Hiding some buttons — reduces functionality.

### Decision 3: Mobile grid — reduce gap and remove outer Card wrapper
- **Approach**: Change `gap: 8` → `gap: 4` in the mobile grid. For "Sin stack" section on mobile, render rows directly without the outer Card.
- **Rationale**: Direct rendering eliminates nested borders and padding. The row's own `borderBottom` provides visual separation.

### Decision 4: Root Layout — remove default border
- **Approach**: Add `style={{ border: 'none', background: 'transparent' }}` to the root `Layout` component.
- **Rationale**: Ant Design's `Layout` can show a subtle outline depending on theme. Explicitly removing it ensures no frame.

## Risks / Trade-offs

- [Low] Removing Card borders on mobile reduces visual hierarchy slightly — mitigated by keeping `borderBottom` on rows
- [Low] Header `overflowX: 'auto'` adds a subtle scrollbar on very narrow screens — acceptable trade-off vs buttons overflowing