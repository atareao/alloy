# Proposal

## Why

The hook message `❌ [PRE-COMMIT] COMMIT RECHAZADO: Cambios en código fuente sin change proposal activo.` does not explain the operational trap: **if you archive the change (`openspec archive <feature> --yes`) BEFORE committing**, the directory `openspec/changes/<feature>/` disappears and the subsequent commit has no active `proposal.md`, so it gets rejected without telling you the correct remediation. This was documented in `docs/pre-commit-hook.md` (section *Trampa*) but users see it only after the failure.

Issue #212.

## What Changes

- Add one explanatory sentence to the error message of `.githooks/pre-commit`: when the commit is rejected, state the cause (archived before commit) and the correct order (`commit → archive`).
- No changes to `require-proposal.sh` or `validate-workflow.sh`.

## Capabilities

### New Capabilities: none.

### Modified Capabilities: none (documentation only).

## Impact

- Documentation only.
- No effect on backend/frontend/tests/deploy.
- Risk: none functional.

## Scenarios

(Optional brief mention.)