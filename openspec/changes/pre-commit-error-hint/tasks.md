# Tasks

## 1. RED

- [ ] 1.1 Verify current error message in `.githooks/pre-commit` contains `COMMIT RECHAZADO` and `proposal`.
- [ ] 1.2 Run the hook locally on a staged source file without an active proposal and capture the current output.

## 2. GREEN

- [ ] 2.1 Edit `.githooks/pre-commit`: add an explanatory line after `COMMIT RECHAZADO...` stating the probable cause (`archivaste el change antes de commitear`) and the correct order (`commit → archive`).
- [ ] 2.2 Re-run the hook on the same scenario and confirm the new message appears.

## 3. Verification

- [ ] 3.1 `just check` (shell syntax + lint) passes.
- [ ] 3.2 `openspec validate --changes` passes (skip_specs: true).
- [ ] 3.3 Hook is installable via `just install-hooks` and functional.