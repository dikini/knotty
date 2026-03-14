# GTK Graph UI and Graph Context

## Metadata
- ID: `COMP-GTK-GRAPH-006`
- Scope: `component`
- Status: `proposed`
- Parent: `SYS-GTK-PARITY-001`
- Concerns: [CAP, REL, UX]
- Created: `2026-03-14`
- Updated: `2026-03-14`

## Purpose
Provide graph functionality in GTK with the same user-facing capabilities as the Tauri app: graph rendering, graph scope control, graph context, and note activation from graph selection.

## Scope

### In Scope
- vault graph and node graph routing
- graph surface using backend-provided layout
- graph selection, hover, and note activation
- graph context controls including reset and node-depth control
- selected-node details, neighbors, and backlinks

### Out of Scope
- pixel-identical rendering with the Tauri SVG implementation
- graph settings pane implementation details beyond required inputs

## Contract

### Functional Requirements

**FR-1**: Graph rendering
- GTK must render nodes and edges from backend layout data.

**FR-2**: Graph interaction
- User can select a node, inspect its related nodes, and open that note.

**FR-3**: Scope control
- Graph mode must support vault scope and node scope.
- Node scope depth must be adjustable.

**FR-4**: Context panel support
- Graph controls and selected-node details must be available in the context panel or equivalent GTK-native region.

**FR-5**: Reset and reframe
- Graph surface must provide reset behavior and consistent framing rules.

## Design Decisions

| Decision | Rationale | Trade-off |
|---|---|---|
| Treat graph as a separate slice after shell | Limits overlap with editor and explorer work | Graph lands later |
| Use GTK-native rendering or embedded drawing as needed | Functionality matters more than matching SVG internals | Rendering stack may differ from Tauri |
| Keep graph context with the shell context panel | Matches existing shell mental model | Requires careful state routing |

## Acceptance Criteria

- [ ] Graph renders nodes and edges from backend data.
- [ ] Node selection updates graph context details.
- [ ] Node activation opens the note.
- [ ] Scope switching and depth control are implemented and tested.
- [ ] Reset behavior and framing rules are documented and tested.

## Related

- Depends on: `COMP-GTK-SHELL-002`
