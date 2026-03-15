# GTK Spec Map

## Metadata
- ID: `SYS-GTK-SPECMAP-001`
- Scope: `system`
- Status: `proposed`
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Provide a compact registry of the GTK parity slice specs, their delivery order, current status, and paired implementation plans.

## Registry

| Order | Spec ID | Title | Status | Depends On | Plan |
|---|---|---|---|---|---|
| 1 | `COMP-GTK-RUNTIME-001` | GTK Runtime Contract and Async Execution | `proposed` | - | `docs/plans/2026-03-14-gtk-runtime-contract-plan.md` |
| 2 | `COMP-GTK-SHELL-002` | GTK App Shell, Startup, Navigation, and Search | `proposed` | `COMP-GTK-RUNTIME-001` | `docs/plans/2026-03-14-gtk-shell-plan.md` |
| 3 | `COMP-GTK-EXPLORER-003` | GTK Explorer Tree and Note Lifecycle | `proposed` | `COMP-GTK-RUNTIME-001` | `docs/plans/2026-03-14-gtk-explorer-plan.md` |
| 4 | `COMP-GTK-EDITOR-004` | GTK Editor Core and Mode Handling | `proposed` | `COMP-GTK-SHELL-002`, `COMP-GTK-EXPLORER-003` | `docs/plans/2026-03-14-gtk-editor-plan.md` |
| 5 | `COMP-GTK-NOTE-TYPES-005` | GTK Note Types, Media, and Embed Surfaces | `proposed` | `COMP-GTK-EDITOR-004` | `docs/plans/2026-03-14-gtk-note-types-plan.md` |
| 6 | `COMP-GTK-GRAPH-006` | GTK Graph UI and Graph Context | `proposed` | `COMP-GTK-SHELL-002` | `docs/plans/2026-03-14-gtk-graph-plan.md` |
| 7 | `COMP-GTK-SETTINGS-007` | GTK Settings, Plugins, and Maintenance | `proposed` | `COMP-GTK-SHELL-002` | `docs/plans/2026-03-14-gtk-settings-plan.md` |
| 8 | `COMP-GTK-AUTOMATION-008` | GTK UI Automation and Parity Harnesses | `proposed` | `COMP-GTK-RUNTIME-001`, `COMP-GTK-SHELL-002`, `COMP-GTK-SETTINGS-007` | `docs/plans/2026-03-14-gtk-automation-plan.md` |
| 9 | `COMP-GTK-DEPRECATIONS-009` | GTK Deprecated API Modernization | `proposed` | `COMP-GTK-EXPLORER-003`, `COMP-GTK-SHELL-002` | `docs/plans/2026-03-15-gtk-deprecations-plan.md` |
| 10 | `COMP-GTK-QUALITY-010` | GTK Code Quality and Test Hardening | `proposed` | `COMP-GTK-EDITOR-004`, `COMP-GTK-NOTE-TYPES-005`, `COMP-GTK-DEPRECATIONS-009` | `docs/plans/2026-03-15-gtk-quality-plan.md` |

## Suggested Parallel Lanes

### Lane A: Foundations
- `COMP-GTK-RUNTIME-001`
- `COMP-GTK-SHELL-002`

### Lane B: Navigation and Editing
- `COMP-GTK-EXPLORER-003`
- `COMP-GTK-EDITOR-004`
- `COMP-GTK-NOTE-TYPES-005`

### Lane C: Secondary Surfaces
- `COMP-GTK-GRAPH-006`
- `COMP-GTK-SETTINGS-007`

### Lane D: Verification
- `COMP-GTK-AUTOMATION-008`

## Status Rules

- Update the spec status first when a slice changes state.
- Keep the paired plan path stable unless the plan is intentionally superseded.
- Add audit or testing artifacts only after the implementation slice has real verification output.

## Related

- Roadmap: `docs/specs/system/gtk-parity-roadmap.md`
- Entry point: `docs/README.md`
- Reference bundle: `docs/reference/README.md`
