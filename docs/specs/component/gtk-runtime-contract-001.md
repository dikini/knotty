# GTK Runtime Contract and Async Execution

## Metadata
- ID: `COMP-GTK-RUNTIME-001`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, DX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Stabilize the GTK frontend data contract and execution model so later slices can build on the same note, graph, settings, and automation payloads as the Tauri UI without freezing the GTK main thread.

## Scope

### In Scope
- align GTK-side Rust DTOs with the Tauri frontend contract where GTK consumes the data
- add missing fields needed by later slices, including note mode and media-related fields
- replace direct blocking RPC work in UI callbacks with background execution plus GTK-safe UI updates
- provide a small reusable request/state pattern for loading, success, and error transitions
- fix CLI/socket-path inconsistencies and establish startup contract tests

### Out of Scope
- full UI automation behavior
- implementing settings panes or graph widgets
- repo migration into `../knot`

## Contract

### Functional Requirements

**FR-1**: Shared payload coverage
- GTK client types must support the payload shape needed by later parity slices.
- At minimum, note payloads must carry `note_type`, `available_modes`, `type_badge`, `media`, `metadata`, and `embed` when present.

**FR-2**: Non-blocking UI execution
- Note loads, searches, explorer refreshes, graph loads, settings loads, and save operations must not block the GTK main loop.
- UI code must receive completion back on the main thread through a consistent pattern.

**FR-3**: Request state model
- Each async UI feature must be able to represent idle, loading, success, and error states without custom ad hoc logging-only behavior.

**FR-4**: Startup contract correctness
- Socket path resolution must match the documented behavior.
- The startup contract must be covered by tests.

**FR-5**: Error propagation
- RPC and daemon errors must be turned into user-visible states that later slices can render, not only log output.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Expand GTK DTOs now | Removes repeated type churn in later slices | Front-loads contract work |
| Background execution wrapper before feature work | Prevents every later slice from inventing its own threading model | Requires a small infrastructure layer |
| Small local request-state abstraction | Enough structure for junior contributors without overengineering | Not a full state-management framework |
| Keep daemon transport | Matches current architecture and avoids repo-integration assumptions | Requires explicit async bridging |

## Acceptance Criteria

- [ ] GTK client DTOs cover all fields required by planned parity slices.
- [ ] UI callbacks no longer perform blocking daemon calls directly on the GTK main thread.
- [ ] A reusable request-state pattern exists and is used by at least one representative flow.
- [ ] CLI path tests pass and document the real startup contract.
- [ ] Errors can be surfaced in UI state instead of only logs.

## Related

- Enables: `COMP-GTK-SHELL-002`, `COMP-GTK-EXPLORER-003`, `COMP-GTK-AUTOMATION-008`
- Current code: `/home/dikini/Projects/knot-gtk/src/client/mod.rs`, `/home/dikini/Projects/knot-gtk/src/main.rs`, `/home/dikini/Projects/knot-gtk/src/cli.rs`
