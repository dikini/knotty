# GTK UI Automation and Parity Harnesses

## Metadata
- ID: `COMP-GTK-AUTOMATION-008`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [REL, DX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Provide the automation hooks and parity harnesses needed to verify GTK feature parity consistently and to expose equivalent UI automation capabilities where they are required by the current system.

## Scope

### In Scope
- stable identifiers for shell views and important widgets
- UI automation state snapshots or GTK-equivalent observability hooks
- parity-focused integration tests and review checklists
- optional settings hookup for automation enablement if needed by contract

### Out of Scope
- redesigning the main repo automation workflow
- repo migration

## Contract

### Functional Requirements

**FR-1**: Stable automation identity
- GTK must expose stable identifiers for major views and controls needed by parity verification.

**FR-2**: Observable state
- GTK must expose enough state to support semantic verification of active view, active note, and critical UI mode.

**FR-3**: Parity harness
- GTK must have a repeatable verification harness covering the completed parity slices.

**FR-4**: Review artifacts
- Each completed slice must have a manual review checklist or equivalent artifact that can be re-run.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Deliver automation last | Avoids redoing identifiers while shell is still moving | Automation coverage lands later |
| Focus on semantic observability over raw widget scraping | Matches the spirit of the Tauri automation design | Requires a deliberate state model |
| Pair automation with parity harness docs | Keeps verification repeatable for junior developers | More documentation work |

## Acceptance Criteria

- [ ] Major GTK views expose stable automation identifiers.
- [ ] GTK can report enough semantic state for parity checks.
- [ ] Integration/parity tests exist for completed slices.
- [ ] Manual review checklists exist for the delivered functionality.

## Related

- Depends on: `COMP-GTK-RUNTIME-001`, `COMP-GTK-SHELL-002`, `COMP-GTK-SETTINGS-007`
