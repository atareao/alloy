# Tasks

## 1. Root Layout — remove visible frame/border

- [x] 1.1 Add `style={{ border: 'none', background: 'transparent' }}` to the root `<Layout>` in App.tsx — verify no visible border around the app in both themes

## 2. Header — fix mobile overflow

- [x] 2.1 Reduce `marginLeft` on username text from 36 to 8 on mobile in App.tsx — verify buttons fit on 375px viewport
- [x] 2.2 Add `overflowX: 'auto'` and `flexShrink: 0` to the button Flex container in App.tsx — verify scrollable if needed
- [x] 2.3 Reduce button gap from 4 to 2 on mobile in App.tsx — verify tighter layout

## 3. ContainerRow — compact mobile rows

- [x] 3.1 Conditionally render `Card` (desktop) vs plain `div` with `borderBottom` (mobile) in ContainerRow.tsx — verify rows stack flush on mobile
- [x] 3.2 Reduce padding from 8px to 4px on mobile in ContainerRow.tsx — verify compact appearance
- [x] 3.3 Add subtle background color to mobile rows to distinguish from page background — verify cells are visible

## 4. ContainerTable — compact mobile grid and "Sin stack" section

- [x] 4.1 Reduce grid `gap` from 8 to 4 in ContainerTable.tsx mobile section — verify tighter spacing
- [x] 4.2 Remove outer Card wrapper for "Sin stack" section on mobile in ContainerTable.tsx — verify no nested borders
- [x] 4.3 Remove `marginBottom: 16` from mobile grid div — verify no extra space below grid

## 5. DashboardPage — stats cards and stack group cleanup

- [x] 5.1 Remove redundant `bordered` from stats Cards in DashboardPage.tsx — verify cleaner look
- [x] 5.2 Reduce stack group padding from 10 to 6 on mobile in DashboardPage.tsx — verify compact stack cards

## 6. Verification

- [x] 6.1 Run `npx vitest run` to confirm all frontend tests still pass
- [x] 6.2 Run `npx tsc --noEmit` to confirm no TypeScript errors
- [x] 6.3 Run `npm run lint` to confirm no linting errors