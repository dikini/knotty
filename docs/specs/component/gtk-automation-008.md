# GTK UI Automation and Parity Harnesses

## Metadata
- ID: `COMP-GTK-AUTOMATION-008`
- Scope: `component`
- Status: `completed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [REL, DX]
- Created: `2026-03-14`
- Updated: `2026-03-16`

## Purpose
Provide the gated, daemon-consumable automation contract and mocked transport harnesses needed to verify GTK feature parity consistently and to let later `knotd` work drive semantic UI state for testing, onboarding, and assisted support flows.

## Scope

### In Scope
- gated daemon-consumable automation protocol for GTK semantic UI control
- stable identifiers for shell views and important widgets
- UI automation state snapshots and discoverable properties
- semantic action discovery and typed action execution results
- parity-focused integration tests and review checklists
- local config plus CLI runtime token gating for automation enablement
- mocked transport coverage for daemon-style callers

### Out of Scope
- text entry or generic button/click primitives
- arbitrary widget addressing or brittle widget-tree automation
- ungated remote control of the GTK UI
- live `knotd` IPC/RPC integration
- repo migration

## Contract

### Functional Requirements

**FR-1**: Stable automation identity
- GTK must expose stable identifiers for major views and controls needed by parity verification and semantic automation.

**FR-2**: Gated automation availability
- GTK automation must be disabled by default.
- GTK automation becomes available only when both are satisfied:
  - local app configuration enables automation
  - the current process is started with a runtime automation token/enable flag
- GTK must surface a visible automation-active indicator when automation is live.

**FR-3**: Observable semantic state
- GTK must expose enough semantic state to support daemon-consumable callers and parity verification of active view, active note, critical UI mode, startup state, settings section, and graph state.

**FR-4**: Discoverable protocol
- GTK automation must provide a discovery surface that reports:
  - protocol version
  - availability and gating status
  - supported snapshot schema version
  - supported action catalog
  - action argument schemas and result codes

**FR-5**: Semantic action execution
- GTK must support semantic UI actions for navigation and mode/state changes without exposing text-entry or generic widget-click primitives in this slice.

**FR-6**: Parity harness
- GTK must have a repeatable verification harness covering the completed parity slices through the same semantic snapshot/action layer.

**FR-7**: Review artifacts
- Each completed slice must have a manual review checklist or equivalent artifact that can be re-run.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Deliver automation after core surfaces | Avoids redoing identifiers while shell/editor/settings are still moving | Automation coverage lands later |
| Use semantic snapshot and action models instead of widget automation | Keeps the contract stable for later `knotd`, tests, and LLM callers | Requires deliberate state projection |
| Gate automation with config and runtime token | Balances safety with developer/support usability | Slightly more startup/config complexity |
| Add discovery and action metadata up front | Lets later `knotd` work integrate without hard-coded assumptions | More protocol documentation work |
| Defer live daemon transport to a later slice | Keeps this slice focused on the GTK contract and mocked harnesses | End-to-end daemon wiring lands later |
| Pair automation with parity harness docs | Keeps verification repeatable for junior developers | More documentation work |

## Acceptance Criteria

- [x] Major GTK views expose stable automation identifiers.
- [x] Automation is unavailable by default and requires both config opt-in and runtime token enablement.
- [x] GTK can report enough semantic state and properties for daemon-consumable callers and parity checks.
- [x] GTK exposes a discoverable action catalog with stable result codes.
- [x] Integration/parity tests exist for completed slices through the automation layer.
- [x] Manual review checklists exist for the delivered functionality.

## Related

- Depends on: `COMP-GTK-RUNTIME-001`, `COMP-GTK-SHELL-002`, `COMP-GTK-SETTINGS-007`
