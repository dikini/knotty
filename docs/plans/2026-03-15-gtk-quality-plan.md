# GTK Code Quality and Test Hardening Plan

## Metadata

- Created: `2026-03-15`
- Scope: `maintenance`
- Status: `approved`
- Spec: `docs/specs/component/gtk-quality-010.md`

## Goal

Make `cargo check` and `cargo clippy --workspace --all-targets --all-features` clean enough to trust again, remove stale code that no longer serves planned slices, and tighten tests around shipped GTK behavior so later slices start from a cleaner baseline.

## Baseline Inventory

Captured from `cargo check` and `cargo clippy --workspace --all-targets --all-features` on `2026-03-15` in a clean worktree branched from `main`.

- `cargo check`: 19 warnings
- `cargo clippy --workspace --all-targets --all-features`: 66 warnings

### `cargo check` warning groups

- `src/cli.rs`
  - unused `vault_path`
- `src/client/mod.rs`
  - unused methods on `KnotdClient`
  - unused DTO structs for tools, settings, notes, and graph payloads
- `src/ui/context_panel.rs`
  - unused methods `refresh` and `connect_mode_changed`
- `src/ui/explorer.rs`
  - unused `NoteSwitchDecision` variants `Deny` and `SaveThenAllow`
- `src/ui/search_view.rs`
  - unread widget/state fields and unused `clear`
- `src/ui/shell_state.rs`
  - unused `ContextMode`, unused `ContentMode::Error`, unused `context_mode`
- `src/ui/window.rs`
  - unused `new` and `widget`

### `cargo clippy` warning groups

- `src/client/mod.rs`
  - repeated `redundant_closure` on `map_err(|e| ClientError::Json(e))`
- `src/ui/context_panel.rs`
  - `type_complexity` callback storage
- `src/ui/editor.rs`
  - `type_complexity`
  - `single_match`
  - `manual_strip`
- `src/ui/note_types.rs`
  - `redundant_guards`
- `src/ui/search_view.rs`
  - `too_many_arguments`
  - `type_complexity`
  - `needless_borrows_for_generic_args`
  - `items_after_test_module`
- `src/ui/window.rs`
  - `too_many_arguments`
  - `type_complexity`
  - `redundant_closure`
  - `needless_borrows_for_generic_args`
  - test-only `type_complexity`

### Expected execution order

1. Remove `cargo check` warnings first, because they overlap with a large part of the clippy inventory.
2. Clean contained clippy categories next, starting with `src/client/mod.rs`, `src/ui/note_types.rs`, and `src/ui/editor.rs`.
3. Tackle structural cleanup in `search_view`, `context_panel`, and `window` after the low-risk warnings are gone.

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTQ-001 | Capture warning and cleanup inventory | - |
| GTQ-002 | Eliminate compile warnings in active codepaths | GTQ-001 |
| GTQ-003 | Eliminate clippy warnings in active codepaths | GTQ-002 |
| GTQ-004 | Simplify high-friction callback and helper patterns | GTQ-003 |
| GTQ-005 | Remove dead code and document intentional leftovers | GTQ-004 |
| GTQ-006 | Tighten tests and verification guidance | GTQ-005 |
| GTQ-007 | Review, fix, and record residual debt | GTQ-006 |

## Plan

### GTQ-001: Capture warning and cleanup inventory

1. Run `cargo check` and `cargo clippy --workspace --all-targets --all-features`.
2. Group warnings by:
   - trivial fix
   - local refactor
   - deferred
3. Cross-check unused code against active plans and `docs/notes/`.
4. Record the concrete baseline counts in the plan before cleanup starts.

### GTQ-002: Eliminate compile warnings in active codepaths

Target dead-code and unused-item warnings first:
- unused fields
- unused methods
- unused helper paths
- stale enum variants and structs

Keep removals narrow and avoid deleting code owned by an unfinished planned slice unless that slice is updated accordingly.

### GTQ-003: Eliminate clippy warnings in active codepaths

Target contained clippy categories next:
- redundant closures
- manual prefix stripping
- needless borrows
- type complexity
- test-module ordering issues
- other active-path warnings that can be fixed without broad rewrites

### GTQ-004: Simplify high-friction callback and helper patterns

Focus on repeated patterns already called out by clippy or review:
- add type aliases where callback types obscure intent
- bundle oversized parameter sets into small context structs
- collapse repeated reset/update paths where helper duplication is already visible

### GTQ-005: Remove dead code and document intentional leftovers

1. Remove unused code that no longer supports a planned slice.
2. For code intentionally retained for future work, record it in `docs/notes/<subsystem>.md`.

### GTQ-006: Tighten tests and verification guidance

1. Strengthen weak assertions in touched areas.
2. Add regressions where cleanup changes behavior.
3. Update workflow docs if the verification story changes.

### GTQ-007: Review, fix, and record residual debt

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
cargo nextest run --workspace
```

## Exit Criteria

- `cargo check` is warning-free for repository code, or every remaining warning is explicitly documented as a temporary exception.
- `cargo clippy --workspace --all-targets --all-features` is warning-free for repository code, or every remaining warning is explicitly documented as a temporary exception.
- Cleanup changes are covered by tests and review.
- Remaining quality debt is explicit and discoverable.
