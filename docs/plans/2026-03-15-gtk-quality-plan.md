# GTK Code Quality and Test Hardening Plan

## Metadata

- Created: `2026-03-15`
- Scope: `maintenance`
- Status: `approved`
- Spec: `docs/specs/component/gtk-quality-010.md`

## Goal

Reduce maintenance noise in `knot-gtk` by cleaning up active clippy warnings, removing stale code that no longer serves planned slices, and tightening tests around shipped GTK behavior.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTQ-001 | Capture warning and cleanup inventory | - |
| GTQ-002 | Fix low-risk clippy issues in active codepaths | GTQ-001 |
| GTQ-003 | Simplify high-friction callback and helper patterns | GTQ-002 |
| GTQ-004 | Remove dead code and document intentional leftovers | GTQ-003 |
| GTQ-005 | Tighten tests and verification guidance | GTQ-004 |
| GTQ-006 | Review, fix, and record residual debt | GTQ-005 |

## Plan

### GTQ-001: Capture warning and cleanup inventory

1. Run `cargo clippy --workspace --all-targets --all-features`.
2. Group warnings by:
   - trivial fix
   - local refactor
   - deferred
3. Cross-check unused code against active plans and `docs/notes/`.

### GTQ-002: Fix low-risk clippy issues in active codepaths

Target low-risk fixes first:
- redundant closures
- manual prefix stripping
- needless borrows
- test-module ordering issues

Keep fixes narrow and avoid unrelated churn.

### GTQ-003: Simplify high-friction callback and helper patterns

Focus on repeated patterns already called out by clippy or review:
- add type aliases where callback types obscure intent
- bundle oversized parameter sets into small context structs
- collapse repeated reset/update paths where helper duplication is already visible

### GTQ-004: Remove dead code and document intentional leftovers

1. Remove unused code that no longer supports a planned slice.
2. For code intentionally retained for future work, record it in `docs/notes/<subsystem>.md`.

### GTQ-005: Tighten tests and verification guidance

1. Strengthen weak assertions in touched areas.
2. Add regressions where cleanup changes behavior.
3. Update any workflow docs if the verification story changes.

### GTQ-006: Review, fix, and record residual debt

1. Review the slice for:
   - spec alignment
   - code smell
   - simplification opportunities
2. Fix all actionable findings.
3. Record remaining deferred cleanup in `docs/notes/`.

## Verification

```bash
cargo fmt --check
cargo check
cargo clippy --workspace --all-targets --all-features
cargo test
```

## Exit Criteria

- The warning surface is materially smaller than the initial inventory.
- Cleanup changes are covered by tests and review.
- Remaining quality debt is explicit and discoverable.
