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

Arrive at a clean enough `knotty` codebase that compile and lint output are trustworthy again, dead or stale code no longer obscures active behavior, and the remaining feature slices can be developed on a substantially cleaner foundation.

## Requirements

**FR-1**: Clippy quality baseline
- Repository code must reach a clean `cargo clippy --workspace --all-targets --all-features` run for the current baseline, unless a warning is explicitly documented as a temporary exception with owner and exit criteria.
- The slice should remove the highest-noise warning categories first, especially warnings caused by obviously redundant closures, manual string slicing, test-module layout issues, avoidable type-complexity hotspots, and unused code in active GTK paths.

**FR-2**: Dead and stale code cleanup
- Unused structs, methods, fields, and helper paths that no longer support an active planned slice must be removed.
- Code that remains intentionally unused because of a future slice must be explicitly documented in `docs/notes/` with a clear pointer to the owning slice.

**FR-3**: Compile-warning baseline
- Repository code must reach a clean `cargo check` run for the current baseline, unless a warning is explicitly documented as a temporary exception with owner and exit criteria.

**FR-4**: Test hardening
- Existing tests must be reviewed for missing coverage around shipped behavior.
- Utility-level and slice-level tests should be tightened where current assertions are too weak or where regressions are likely.
- Flaky or redundant tests should be simplified or replaced.

**FR-5**: Gate tightening roadmap
- The repo must define the path from “clippy runs in the gate” to “the local gate can enforce a clean baseline.”
- If the slice cannot safely raise enforcement all the way to `-D warnings`, the remaining blockers must be explicit and narrow.

**FR-6**: Simplicity and maintainability
- Cleanup work should prefer small local simplifications over broad rewrites.
- Review should explicitly target code smell, over-complex callback/state plumbing, and duplicated helper logic.

## Acceptance

- [ ] `cargo check` emits no repository-code warnings, or every remaining warning is explicitly documented as a temporary exception with owner and exit criteria.
- [ ] `cargo clippy --workspace --all-targets --all-features` emits no repository-code warnings, or every remaining warning is explicitly documented as a temporary exception with owner and exit criteria.
- [ ] Stale or dead code removed by this slice is not needed by an already planned feature slice.
- [ ] Test coverage is tightened for the changed areas, with green local verification.
- [ ] Remaining cleanup debt is captured in `docs/notes/` with clear follow-up rationale.

## Non-Goals

- Completing any unfinished user-facing feature slice.
- Broad architectural rewrites without a clear warning/test payoff.
- Raising the gate straight to `cargo clippy -- -D warnings` before the repo is ready.

## Notes

- This slice is about trust in the codebase, not new UI surface area.
- It should leave the next feature slices with compile and lint output that is substantially easier to read and trust.
