# Tasks

## TDD Checklist

### 1. Fix TypeScript error in App.test.tsx
- [x] Replace `delete window.location` + `window.location = { href: "" }` with `Object.defineProperty`
- [x] Replace `window.location = originalLocation` with `Object.defineProperty` restore
- [x] Run `npx tsc -b` to verify zero errors
- [x] Run `npx vitest run` to verify tests still pass