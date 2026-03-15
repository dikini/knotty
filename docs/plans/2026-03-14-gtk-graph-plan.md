# GTK Graph UI and Graph Context Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** deliver graph parity in GTK with vault/node scope, graph context details, and note activation from graph selection.

**Architecture:** keep graph state isolated from editor internals, use backend-provided layout for the vault graph, normalize the daemon neighborhood payload into the same internal scene model, and route graph context through the shell context panel rather than inventing a separate graph shell.

**Tech Stack:** Rust, gtk4, libadwaita, drawing/widget toolkit chosen by implementation, cargo test

---

## How To Use This Plan

- Read the shared playbook first: `docs/plans/2026-03-14-gtk-parity-execution-playbook.md`
- Read the local references: `docs/reference/graph-behavior.md`, `docs/reference/shell-behavior.md`, and `docs/reference/knotd-protocol.md`
- Keep graph state isolated from editor state.
- Route note activation back through the shell rather than loading notes inside the graph widget.

## Delivery Notes

- Avoid mixing graph rendering and settings work in the same task.
- Keep graph-specific state local to graph modules and shell routing.

## Rust Guidance For This Slice

- Keep graph data models close to the daemon payload.
- Prefer pure helpers for selection and scope calculations.
- Separate rendering from interaction state when practical.

## knotd Calls Used By This Slice

- `get_graph_layout`
- `graph_neighbors`
- `get_note` when a graph node is activated

### Graph layout types

```rust
pub struct GraphLayout {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
}

pub struct GraphEdge {
    pub source: String,
    pub target: String,
}
```

### Focused neighborhood type

```rust
pub struct GraphNeighborhood {
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}
```

### Graph request examples

```json
{
  "name": "get_graph_layout",
  "arguments": {
    "width": 900.0,
    "height": 600.0
  }
}
```

```json
{
  "name": "graph_neighbors",
  "arguments": {
    "path": "notes/example.md",
    "depth": 1
  }
}
```

### Graph communication sequence

1. user enters graph mode
2. graph view requests layout from daemon
3. graph view renders nodes and edges
4. selecting a node updates local graph selection state
5. context panel derives selected-node details from the current normalized scene
6. activating a node delegates back to shared note loading

## Task Summary

| ID | Task | Depends |
|---|---|---|
| GTG-001 | Add tests for graph shell routing and load states | - |
| GTG-002 | Implement graph surface with backend layout data | GTG-001 |
| GTG-003 | Add node selection and note activation | GTG-002 |
| GTG-004 | Add graph context details, neighbors, and backlinks | GTG-003 |
| GTG-005 | Add scope switching, depth control, and reset | GTG-004 |
| GTG-006 | Full verification and review fixes | GTG-005 |

## Small Task Breakdown

| Small ID | Parent | Action | Primary Files | Do Not Touch |
|---|---|---|---|---|
| GTG-001A | GTG-001 | Add failing graph-mode shell-routing test | `src/ui/window.rs`, `src/ui/context_panel.rs` | editor files |
| GTG-001B | GTG-001 | Add failing graph loading/error state test | `src/ui/window.rs` | settings files |
| GTG-001C | GTG-001 | Implement graph shell wiring | `src/ui/window.rs`, `src/ui/context_panel.rs` | `src/ui/editor.rs` |
| GTG-002A | GTG-002 | Add failing node-render test | `src/ui/graph_view.rs` | shell state |
| GTG-002B | GTG-002 | Add failing edge-render test | `src/ui/graph_view.rs` | shell state |
| GTG-002C | GTG-002 | Add failing empty-layout test | `src/ui/graph_view.rs` | shell state |
| GTG-002D | GTG-002 | Implement minimal graph renderer | `src/ui/graph_view.rs`, `src/ui/mod.rs` | note loading |
| GTG-003A | GTG-003 | Add failing node-selection test | `src/ui/graph_view.rs` | context details |
| GTG-003B | GTG-003 | Add failing node-activation test | `src/ui/graph_view.rs`, `src/ui/window.rs` | settings files |
| GTG-003C | GTG-003 | Implement selection and activation callbacks | `src/ui/graph_view.rs`, `src/ui/window.rs` | editor internals |
| GTG-004A | GTG-004 | Add failing selected-node details test | `src/ui/context_panel.rs` | renderer internals |
| GTG-004B | GTG-004 | Add failing neighbors/backlinks list test | `src/ui/context_panel.rs` | renderer internals |
| GTG-004C | GTG-004 | Implement graph context details | `src/ui/context_panel.rs` | editor/settings |
| GTG-005A | GTG-005 | Add failing scope-switch test | `src/ui/graph_view.rs`, `src/ui/context_panel.rs` | shell state |
| GTG-005B | GTG-005 | Add failing depth-change test | `src/ui/graph_view.rs`, `src/ui/context_panel.rs` | shell state |
| GTG-005C | GTG-005 | Add failing reset test | `src/ui/graph_view.rs` | shell state |
| GTG-005D | GTG-005 | Implement scope/depth/reset behavior | touched graph files only | editor/settings |
| GTG-006A | GTG-006 | Run slice verification | repo-wide | - |
| GTG-006B | GTG-006 | Fix slice-only regressions | touched files only | unrelated modules |

### Task GTG-001: Add tests for graph shell routing and load states

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`

**Steps**
1. Write failing tests for entering graph mode, showing graph context, and handling loading/error states.
2. Confirm red.
3. Implement the minimal shell wiring needed for graph mode.
4. Re-run targeted tests until green.
5. Review for shell-state duplication.

**Advice**

- The graph view should not decide global shell routing rules.
- It should only report events upward.
- Focused-neighborhood payloads should be normalized before they reach rendering or context logic.

### Task GTG-002: Implement graph surface with backend layout data

**Files**
- Create or modify: `/home/dikini/Projects/knot-gtk/src/ui/graph_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/mod.rs`

**Steps**
1. Write failing tests for rendering nodes and edges from a graph layout payload.
2. Confirm red.
3. Implement the minimal graph renderer.
4. Re-run targeted tests until green.
5. Review rendering technology choice and document it in the module.

**Example test ideas**

- empty graph layout shows empty state
- graph layout with two nodes creates two selectable targets
- edge count matches payload

### Task GTG-003: Add node selection and note activation

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/graph_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/window.rs`

**Steps**
1. Write failing tests for node selection and note-open activation.
2. Confirm red.
3. Implement the selection and activation flow.
4. Re-run targeted tests until green.
5. Review state ownership so the graph does not directly own note loading logic.

**Example callback sketch**

```rust
pub enum GraphEvent {
    NodeSelected(String),
    NodeActivated(String),
    ResetRequested,
}
```

### Task GTG-004: Add graph context details, neighbors, and backlinks

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/graph_view.rs`

**Steps**
1. Add failing tests for selected-node details, neighbors, and backlinks rendering.
2. Confirm red.
3. Implement the minimal context detail surface.
4. Re-run targeted tests until green.
5. Review empty-state and no-selection behavior.

**Advice**

- The context panel should behave sensibly when nothing is selected.
- Tests should cover both selected and unselected graph states.
- Prefer deriving neighbors and backlinks from the normalized scene instead of inventing a second graph-details payload.

### Task GTG-005: Add scope switching, depth control, and reset

**Files**
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/graph_view.rs`
- Modify: `/home/dikini/Projects/knot-gtk/src/ui/context_panel.rs`

**Steps**
1. Write failing tests for vault scope, node scope, depth changes, and reset behavior.
2. Confirm red.
3. Implement the minimal controls and framing rules.
4. Re-run targeted tests until green.
5. Review for clear naming around graph scope state.

**Advice**

- Keep scope and depth state in one model.
- Avoid separate booleans like `is_node_scope` when an enum is clearer.
- Reset should restore vault scope and depth `1`.

### Task GTG-006: Full verification and review fixes

**Steps**
1. Run `cargo fmt`.
2. Run targeted graph tests.
3. Run `cargo test`.
4. Manual smoke-check graph selection if a GTK environment is available.
5. Fix review findings.

## Slice Verification

```bash
cargo fmt --check
cargo test
cargo check
```

## Review Checklist

- Graph routing stays in shell state
- Graph selection updates context details correctly
- Node activation goes through shared note-load path
- Scope/depth/reset behavior is explicit and tested
- Rendering code and state code are not tangled together without need

## Commit Gate

Commit only when all verification commands are green.
