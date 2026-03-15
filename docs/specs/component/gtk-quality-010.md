# GTK Code Quality and Test Hardening

## Metadata

- ID: `COMP-GTK-QUALITY-010`
- Scope: `component`
- Status: `proposed`
- Created: `2026-03-15`
- Depends On:
  - `COMP-GTK-EDITOR-004`
  - `COMP-GTK-NOTE-TYPES-005`
  - `COMP-GTK-DEPRECATIONS-009`

## Purpose

Improve repository health after the core GTK parity slices by reducing warning noise, removing stale code paths, tightening test coverage, and making the local verification bar stricter and more trustworthy.

## Requirements

**FR-1**: Clippy quality baseline
- Repository code must move toward a clean `cargo clippy --workspace --all-targets --all-features` run.
- The slice should remove or document the highest-noise warning categories first, especially warnings caused by obviously redundant closures, manual string slicing, test-module layout issues, and avoidable type-complexity hotspots.

**FR-2**: Dead and stale code cleanup
- Unused structs, methods, fields, and helper paths that no longer support an active planned slice must be removed.
- Code that remains intentionally unused because of a future slice must be explicitly documented in `docs/notes/`.

**FR-3**: Test hardening
- Existing tests must be reviewed for missing coverage around shipped behavior.
- Utility-level and slice-level tests should be tightened where current assertions are too weak or where regressions are likely.
- Flaky or redundant tests should be simplified or replaced.

**FR-4**: Gate tightening roadmap
- The repo must define a path from “clippy runs in the gate” to “clippy is clean enough to raise the enforcement bar.”
- The final bar does not need to be `-D warnings` in this slice, but the remaining blockers must be explicit.

**FR-5**: Simplicity and maintainability
- Cleanup work should prefer small local simplifications over broad rewrites.
- Review should explicitly target code smell, over-complex callback/state plumbing, and duplicated helper logic.

## Acceptance

- [ ] `cargo clippy --workspace --all-targets --all-features` still runs successfully and emits materially fewer warnings than the pre-slice baseline.
- [ ] High-noise clippy warnings introduced by repository code in active GTK paths are reduced or tracked explicitly.
- [ ] Stale or dead code removed by this slice is not needed by an already planned feature slice.
- [ ] Test coverage is tightened for the changed areas, with green local verification.
- [ ] Remaining cleanup debt is captured in `docs/notes/` with clear follow-up rationale.

## Non-Goals

- Completing any unfinished user-facing feature slice.
- Broad architectural rewrites without a clear warning/test payoff.
- Raising the gate straight to `cargo clippy -- -D warnings` before the repo is ready.

## Notes

- This slice is about trust in the codebase, not new UI surface area.
- It should be scheduled after the most important parity slices, but before the repo settles into long-term maintenance with warning noise still masking regressions.
