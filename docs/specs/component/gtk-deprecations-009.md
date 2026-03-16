# GTK Deprecated API Modernization

## Metadata
- ID: `COMP-GTK-DEPRECATIONS-009`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [REL, CAP, MAINT]
- Created: `2026-03-15`
- Updated: `2026-03-15`

## Purpose
Remove deprecated GTK and libadwaita API usage from `knotty`, keep future work off deprecated surfaces, and restore deprecation warnings as a meaningful maintenance signal instead of background noise.

## Scope

### In Scope
- all compile-time GTK and libadwaita deprecation warnings emitted from repository code
- explorer migration away from deprecated tree widgets
- remaining deprecated dialog, list, model, and widget APIs in active codepaths
- project policy and verification guidance for avoiding deprecated APIs

### Out of Scope
- non-GTK Rust warnings
- speculative migrations for APIs that are not currently deprecated
- upstream daemon protocol changes

## Contract

### Functional Requirements

**FR-1**: No active deprecated GTK/libadwaita API usage
- Repository code must not rely on deprecated GTK or libadwaita APIs when a supported replacement exists in the current project baseline.

**FR-2**: Explorer modernization
- The explorer must migrate away from deprecated tree widgets and preserve current explorer behavior: rendering, selection, expansion persistence, and mutation refresh flows.

**FR-3**: Dialog and picker modernization
- Any deprecated dialog or file-selection API still in active use must be replaced with the current supported GTK/libadwaita alternative.

**FR-4**: Policy visibility
- Project documentation must state that new deprecated GTK/libadwaita API usage is not allowed when a supported replacement exists.

**FR-5**: Regression signal
- Verification guidance must treat GTK/libadwaita deprecation warnings in repository code as a failure signal for this slice.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Use one modernization slice with explicit subsystem tasks | Keeps the maintenance goal clear while still allowing scoped execution | Larger plan surface |
| Treat `cargo check` deprecation warnings as the repo signal | Prevents silent drift back onto deprecated APIs | Requires some broader refactors |
| Migrate explorer as part of this slice, not opportunistically | Explorer currently dominates warning noise and touches selection architecture | Higher initial cost |

## Acceptance Criteria

- [ ] `cargo check` emits no GTK/libadwaita deprecation warnings from repository code.
- [ ] Explorer behavior is preserved after migration to non-deprecated GTK list/tree APIs.
- [ ] Project docs explicitly state the no-new-deprecated-APIs policy.
- [ ] Any deferred exception is documented with rationale and exit criteria.

## Related

- Depends on: `COMP-GTK-EXPLORER-003`, `COMP-GTK-SHELL-002`
- Enables: cleaner follow-on work in `COMP-GTK-GRAPH-006`, `COMP-GTK-SETTINGS-007`, and `COMP-GTK-AUTOMATION-008`
